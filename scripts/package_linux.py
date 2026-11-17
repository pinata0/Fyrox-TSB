"""Package the Linux x64 release, preserving executable bits in a tar.gz archive.

Build first with scripts/Build-Linux.ps1. Requires Python 3.11+.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import shutil
import tarfile
import tomllib

from package_portable import ROOT, digest, runtime_assets

EXECUTABLES = {'tsb', 'tsb-server', 'tsb-eos-check', 'tsb-eos-peer-check', 'Play.sh'}
LAUNCHER = '''#!/bin/sh
set -eu
cd -- "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
exec ./tsb "$@"
'''
README = '''Trickcal : Shooting Balddagu — Linux x64

실행
1. 압축 파일 전체를 쓰기 가능한 폴더에 풀어 주세요.
2. 압축을 푼 폴더에서 터미널을 열고 ./Play.sh 를 실행하세요.
   권한 오류가 나면 chmod +x Play.sh tsb tsb-server tsb-eos-check tsb-eos-peer-check
   를 한 번 실행한 다음 ./Play.sh 를 다시 실행하세요.
3. 로그인 또는 게스트 선택 → 홈 → 플레이에서 방을 만들거나 참가합니다.
   혼자 연습하려면 ./Play.sh --solo 를 실행하세요.

지원 환경
- Intel/AMD 64비트 Linux (x86_64), glibc 2.35 이상, OpenGL 3.3 이상.
- Ubuntu 22.04를 기준으로 빌드했습니다. ARM Linux와 macOS용 파일은 아닙니다.
- 그래픽 데스크톱과 해당 GPU 드라이버가 필요합니다.
- ALSA, X11/Wayland, xkbcommon 등 데스크톱 런타임 라이브러리가 필요합니다.
- libxkbcommon-x11.so 오류가 나면 Ubuntu/Debian에서
  sudo apt install libxkbcommon-x11-0
  명령으로 해당 라이브러리를 설치하세요.
- Rust, 컴파일러, EOS SDK 원본 설치는 필요 없습니다.

멀티플레이
- 기존 Windows판과 같은 게임용 EOS 설정 및 Linux EOS 라이브러리를 포함합니다.
- 호스트: 플레이 → 방 만들기 → 방 코드 복사. 참가자: 방 참여하기 → 코드 입력.
- 기기 계정은 이 PC에서 새로 생성되며 Windows 기기 계정은 복사하지 않습니다.
- Windows/Linux 간 실제 두 PC 멀티플레이는 별도 확인이 필요합니다.
- 직접 UDP 접속도 ./Play.sh --host 0.0.0.0:28080 또는
  ./Play.sh --join 호스트IP:28080 으로 사용할 수 있습니다.

진단
- ./tsb-eos-check --offline : SDK 로딩과 초기화만 확인
- ./tsb-eos-check --config eos.local.json : EOS 로그인 및 NAT 확인
- 실행 로그: artifacts/logs/
- 모델, 텍스처, data, libEOSSDK-Linux-Shipping.so, eos.local.json을 함께 유지하세요.
- 파일 무결성 목록: package-manifest.json / 의존성 고지: licenses/
'''


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--name', default='TSB-Linux-x64')
    parser.add_argument('--include-eos-config', action='store_true', required=True)
    parser.add_argument('--stage-only', action='store_true')
    parser.add_argument('--archive-only', action='store_true')
    args = parser.parse_args()
    if not re.fullmatch(r'[A-Za-z0-9_-]+', args.name):
        raise SystemExit('Invalid package name')
    stage = ROOT / 'artifacts/releases' / args.name
    archive = ROOT / 'release' / f'{args.name}.tar.gz'
    if not args.archive_only:
        if stage.exists() or archive.exists():
            raise SystemExit('Package already exists; choose another --name')
        config = ROOT / 'eos.local.json'
        value = json.loads(config.read_text(encoding='utf-8-sig'))
        required = {'product_id', 'sandbox_id', 'deployment_id', 'client_id', 'client_secret'}
        if set(value) != required or not all(isinstance(value[k], str) and value[k].strip() for k in required):
            raise SystemExit('EOS configuration is incomplete (values are not displayed)')
        stage.mkdir(parents=True)

        def copy(source: Path, relative: str | Path) -> None:
            if not source.is_file():
                raise SystemExit(f'Missing runtime file: {source.name}')
            destination = stage / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)

        for name in sorted(EXECUTABLES - {'Play.sh'}):
            source = ROOT / 'target/linux-x64/release' / name
            with source.open('rb') as binary:
                if binary.read(5) != b'\x7fELF\x02':
                    raise SystemExit(f'Expected a Linux ELF64 binary: {name}')
            copy(source, name)
        copy(ROOT / 'target/linux-x64/release/libEOSSDK-Linux-Shipping.so', 'libEOSSDK-Linux-Shipping.so')
        copy(config, 'eos.local.json')
        sdks = list((ROOT.parent / 'tools').glob('EOS-SDK-*/ThirdPartyNotices/ThirdPartySoftwareNotice.txt'))
        if len(sdks) != 1:
            raise SystemExit('Expected one EOS SDK notice file')
        copy(sdks[0], 'licenses/EOS-ThirdPartySoftwareNotice.txt')
        selected = runtime_assets()
        for source in sorted(selected):
            copy(source, source.relative_to(ROOT))
        raw_registry = (ROOT / 'data/resources.registry').read_text(encoding='utf-8-sig')
        registry = json.loads(re.sub(r',\s*([}\]])', r'\1', raw_registry))
        registry = {key: value for key, value in registry.items() if ROOT / value in selected}
        (stage / 'data/characters').mkdir(parents=True)
        (stage / 'data/resources.registry').write_text(json.dumps(registry, ensure_ascii=False, indent=2), encoding='utf-8', newline='\n')
        for package in tomllib.loads((ROOT / 'Cargo.lock').read_text(encoding='utf-8'))['package']:
            if not package.get('source', '').startswith('registry+'):
                continue
            ident = f"{package['name']}-{package['version']}"
            for registry_root in (Path.home() / '.cargo/registry/src').glob('*'):
                directory = registry_root / ident
                if directory.is_dir():
                    for license_file in directory.iterdir():
                        if license_file.is_file() and license_file.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE')):
                            copy(license_file, Path('licenses/rust') / ident / license_file.name)
                    break
        (stage / 'Play.sh').write_text(LAUNCHER, encoding='utf-8', newline='\n')
        (stage / 'README-KO.txt').write_text(README, encoding='utf-8', newline='\n')
        files = [dict(path=p.relative_to(stage).as_posix(), bytes=p.stat().st_size, sha256=digest(p))
                 for p in sorted(stage.rglob('*')) if p.is_file()]
        manifest = dict(name=args.name, platform='Linux x86_64 / glibc >= 2.35', cargo_profile='release',
                        eos_config_included=True, source_code_included=False, files=files)
        (stage / 'package-manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding='utf-8')
        print(f'Staged {len(files)} files: {stage}')

    if not args.stage_only:
        if archive.exists():
            raise SystemExit('Archive already exists; it will not be overwritten')
        manifest = json.loads((stage / 'package-manifest.json').read_text(encoding='utf-8'))
        for item in manifest['files']:
            if digest(stage / item['path']) != item['sha256']:
                raise SystemExit(f'Stage checksum mismatch: {item["path"]}')
        archive.parent.mkdir(parents=True, exist_ok=True)
        pending = archive.with_suffix('.gz.part')
        if pending.exists():
            raise SystemExit('Pending archive exists; inspect it before retrying')

        def permissions(info: tarfile.TarInfo) -> tarfile.TarInfo:
            info.uid = info.gid = 0
            info.uname = info.gname = ''
            relative = Path(info.name).relative_to(args.name).as_posix()
            info.mode = 0o755 if info.isdir() or relative in EXECUTABLES else 0o644
            return info

        with tarfile.open(pending, 'w:gz', compresslevel=6, format=tarfile.PAX_FORMAT) as tar:
            for relative in [item['path'] for item in manifest['files']] + ['package-manifest.json']:
                tar.add(stage / relative, arcname=f'{args.name}/{relative}', filter=permissions)
        # Read back every member; check content hashes and Linux execute permissions.
        with tarfile.open(pending, 'r:gz') as tar:
            import hashlib
            expected = {f'{args.name}/{item["path"]}': item for item in manifest['files']}
            seen = set()
            for member in tar:
                if Path(member.name).relative_to(args.name).as_posix() in EXECUTABLES and member.mode != 0o755:
                    raise SystemExit(f'Execute permission lost: {member.name}')
                if member.name in expected:
                    with tar.extractfile(member) as stream:
                        if hashlib.file_digest(stream, 'sha256').hexdigest() != expected[member.name]['sha256']:
                            raise SystemExit(f'Archive checksum mismatch: {member.name}')
                    seen.add(member.name)
            if seen != expected.keys():
                raise SystemExit('Archive file list mismatch')
        pending.rename(archive)
        checksum = digest(archive)
        archive.with_name(archive.name + '.sha256').write_text(f'{checksum}  {archive.name}\n', encoding='ascii')
        print(f'Ready: {archive} ({archive.stat().st_size / 1048576:.1f} MiB)')
        print(f'SHA256 {checksum}')


if __name__ == '__main__':
    main()
