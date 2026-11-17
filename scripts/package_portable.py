"""Create an explicitly requested Windows test bundle without source/build caches.

Usage: python scripts/package_portable.py --include-eos-config
Build the current executables first with cargo build --locked --offline --bins.
Credentials are copied as bytes, never printed or embedded in the launcher.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SLUGS = "vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion".split()

LAUNCHER = r'''param(
    [ValidateSet('Check','RelayHost','RelayJoin')]
    [string]$Mode = 'Check',
    [string]$RoomCode
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$gameRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $gameRoot
try {
    if (-not (Test-Path -LiteralPath './eos.local.json')) {
        throw 'eos.local.json 파일이 없습니다. ZIP 전체를 새 폴더에 압축 해제해 주세요.'
    }
    if ($Mode -eq 'RelayJoin') {
        if (-not $RoomCode) { $RoomCode = Read-Host '진단 호스트가 출력한 eos:... 코드를 붙여 넣어 주세요' }
        $RoomCode = $RoomCode.Trim()
        if ($RoomCode -cnotmatch '^eos:[a-fA-F0-9]{32}:[a-fA-F0-9]{32}$') {
            throw '이 실행기는 eos: 로 시작하는 전체 방 코드를 사용합니다. 웹의 8자리 코드는 게임 화면에서 입력하세요.'
        }
    }
    $gameExe = Join-Path $gameRoot 'tsb.exe'
    $gameArgs = @()
    switch ($Mode) {
        'Check' { $gameExe = Join-Path $gameRoot 'tsb-eos-check.exe'; $gameArgs = @('--config','eos.local.json') }
        'RelayHost' { $gameExe = Join-Path $gameRoot 'tsb-eos-peer-check.exe'; $gameArgs = @('--host','--force-relay','--seconds','300') }
        'RelayJoin' { $gameExe = Join-Path $gameRoot 'tsb-eos-peer-check.exe'; $gameArgs = @('--join',$RoomCode,'--force-relay','--seconds','300') }
    }
    & $gameExe @gameArgs
    if ($LASTEXITCODE -ne 0) { throw '실행이 완료되지 않았습니다. 위 오류 메시지와 artifacts/logs 폴더를 확인해 주세요.' }
    if ($Mode -in @('Check','RelayHost','RelayJoin')) { [void](Read-Host '확인 후 Enter를 누르면 닫힙니다') }
} catch {
    Write-Host $_.Exception.Message -ForegroundColor Red
    [void](Read-Host '확인 후 Enter를 누르면 닫힙니다')
    exit 1
} finally { Pop-Location }
'''

README = '''Trickcal : Shooting Balddagu — Windows x64 테스트용 ZIP

1. ZIP을 전부 압축 해제하세요. ZIP 내부에서 바로 실행하지 마세요.
2. Play.cmd 하나만 더블클릭하고 게임 안에서 로그인하거나 게스트를 선택하세요.
   Rust, Visual Studio, EOS SDK 원본 설치는 필요 없습니다.
3. Windows 10/11 64비트와 OpenGL을 지원하는 그래픽 드라이버가 필요합니다.
   필요한 Visual C++ 런타임 DLL은 게임 폴더에 포함했습니다.

로그인과 프로필
- 게임 안에서 기존 계정의 이메일·비밀번호로 로그인합니다.
- 홈 → 프로필에서 사진 파일 선택·미리보기·저장·삭제와 프로필 편집이 가능합니다.
- 사진은 JPG·PNG·WebP, 2MB·1,600만 화소 이하를 지원합니다.
- 회원가입과 이메일 확인은 DOTG 웹사이트에서 진행합니다.

웹사이트 없이 친구와 접속하기
- 호스트: Play.cmd → 게임 홈의 플레이 → 방 만들기 → ‘코드 복사하기’ 버튼 클릭.
  로비에 표시된 eos:... 전체 코드가 복사됩니다. 참가자에게 전달하세요.
- 참가자: Play.cmd → 플레이 → 방 참여하기 → 방 코드 입력란에서 Ctrl+V → 참여하기.
- 호스트가 로비에서 게임 시작을 누른 뒤 참가자는 게임 참가를 누릅니다.
- 양쪽 모두 이 ZIP의 동일한 빌드를 사용하세요. 공유기 포트포워딩은 필요 없습니다.
- 혼자 연습: 게임 홈의 전투훈련 → 로비에서 맵/봇 선택 → 게임 시작.
- Host.cmd, Join.cmd, Solo.cmd는 필요 없습니다. 모든 선택은 게임 안에서 합니다.
- 웹 계정 없이 방 만들기와 EOS 코드 참가가 가능합니다. 웹 방 목록에 연결되지 않아도 됩니다.
- 웹 방 목록/8자리 코드는 웹 서버 배포 후 웹 계정을 연결해 사용합니다.
  기본 웹 주소는 https://www.dotgsince2023.co.kr/play이며 다른 웹사이트를 사용할 때만 DOTG_PORTAL_URL을 설정합니다.

EOS 설정
- 요청에 따라 이 빌드의 게임용 eos.local.json을 포함했습니다. 양쪽에서 그대로 사용하세요.
- Client를 PC마다 새로 만들지 않습니다. 사용자 ID는 각 PC에서 자동 생성됩니다.
- EOS 로그인 저장소나 다른 PC의 기기 계정 데이터는 복사하지 않았습니다.
- 같은 PC에서 두 프로세스를 실행하면 같은 EOS 사용자 ID이므로 두 PC 테스트가 되지 않습니다.
- 이 ZIP은 EOS 설정을 포함한 테스트용 공유본입니다.

접속 진단
- diagnostics/Check-EOS.cmd: EOS 로그인과 NAT 확인. 상대 PC 접속 성공을 뜻하지는 않습니다.
- diagnostics/Check-Relay-Host.cmd / Check-Relay-Join.cmd: 서로 다른 PC에서 실행합니다.
  호스트 창의 HOST_READY 다음 eos:... 코드를 참가자에게 전달하세요.
  양쪽 PASS 및 EOS Relay가 나오면 실제 중계 접속을 확인한 것입니다.
- 일반 연결은 직접 연결을 먼저 시도하고 필요하면 Epic 중계를 사용합니다.
- 다른 PC의 실제 중계 접속은 아직 별도 검증이 필요합니다.

주요 조작: WASD 이동 / 마우스 시점 / 왼쪽 클릭 공격 / E 스킬 / Q 궁극기 /
H 캐릭터 선택 / Enter 채팅 / M 마이크 켜기·끄기(기본 음소거) / Esc 메뉴 / F10 종료.

assets, data, DLL 파일과 eos.local.json은 실행 파일과 함께 유지하세요.
로그와 화면 캡처는 실행 시 artifacts 아래에 생성됩니다.
파일 무결성 목록: package-manifest.json
'''


def digest(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def runtime_assets() -> set[Path]:
    """The same models, maps, textures and UI assets on every desktop platform."""
    selected: set[Path] = set()
    for folder in ('assets/characters', 'assets/maps'):
        selected.update(p for p in (ROOT / folder).rglob('*') if p.is_file() and p.suffix.lower() in {'.fbx','.png','.jpg','.jpeg','.dds','.tga','.json'})
    for slug in SLUGS:
        selected.add(ROOT / f'assets/character-inputs/inputs-masked/{slug}.png')
    selected.update((ROOT / 'assets/fonts').glob('*.ttf'))
    selected.update((ROOT / 'assets/ui/markers').glob('*.png'))
    selected.update((ROOT / 'assets/ui/round-results').glob('*.png'))
    selected.update((ROOT / 'assets/ui/startup').glob('*.png'))
    selected.update(ROOT / p for p in ('assets/ui/home/elias-picnic-soft.png', 'assets/ui/home/trickcal-shooting-balddagu-logo.png'))
    return selected


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--include-eos-config', action='store_true', required=True,
                        help='Explicitly authorize copying the game-client EOS configuration')
    parser.add_argument('--profile', choices=('debug', 'release'), default='debug')
    parser.add_argument('--name', default='TSB-Windows-x64-20261113-v2')
    parser.add_argument('--stage-only', action='store_true')
    parser.add_argument('--archive-only', action='store_true')
    args = parser.parse_args()
    if not re.fullmatch(r'[A-Za-z0-9_-]+', args.name):
        raise SystemExit('Invalid package name')
    output = ROOT / 'artifacts' / 'releases'
    stage = output / args.name
    archive = output / f'{args.name}.zip'

    if not args.archive_only:
        if stage.exists() or archive.exists():
            raise SystemExit('Package already exists; choose a new --name instead of overwriting')
        config = ROOT / 'eos.local.json'
        value = json.loads(config.read_text(encoding='utf-8-sig'))
        required = {'product_id', 'sandbox_id', 'deployment_id', 'client_id', 'client_secret'}
        if set(value) != required or not all(isinstance(value[k], str) and value[k].strip() for k in required):
            raise SystemExit('EOS configuration is incomplete (values are not displayed)')
        sdk_candidates = list((ROOT.parent / 'tools').glob('EOS-SDK-*/SDK'))
        if len(sdk_candidates) != 1:
            raise SystemExit('Expected one local EOS SDK')
        sdk = sdk_candidates[0]
        crt_base = Path('C:/Program Files/Microsoft Visual Studio/18/Community/VC/Redist/MSVC')
        crt_candidates = sorted(crt_base.glob('*/x64/Microsoft.*.CRT'),
                                key=lambda p: tuple(int(x) for x in p.parts[-3].split('.')) if p.parts[-3][0].isdigit() else (0,))
        if not crt_candidates:
            raise SystemExit('Microsoft x64 redistributable CRT files were not found')
        stage.mkdir(parents=True)

        def copy(source: Path, relative: str | Path) -> None:
            if not source.is_file():
                raise SystemExit(f'Missing runtime file: {source.name}')
            destination = stage / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)

        for filename in ('tsb.exe', 'tsb-eos-check.exe', 'tsb-eos-peer-check.exe', 'EOSSDK-Win64-Shipping.dll'):
            copy(ROOT / 'target' / args.profile / filename, filename)
        copy(config, 'eos.local.json')
        for dll in crt_candidates[-1].glob('*.dll'):
            copy(dll, dll.name)
        copy(sdk / 'Bin/x64/xaudio2_9redist.dll', 'xaudio2_9redist.dll')
        copy(sdk.parent / 'ThirdPartyNotices/ThirdPartySoftwareNotice.txt', 'licenses/EOS-ThirdPartySoftwareNotice.txt')

        # Runtime assets only: model/weapon meshes, their textures, portraits,
        # both maps, menu art, icons and the bundled Korean font.
        selected = runtime_assets()
        registry_text = (ROOT / 'data/resources.registry').read_text(encoding='utf-8-sig')
        registry = json.loads(re.sub(r',\s*([}\]])', r'\1', registry_text))
        # Retain UUID lookups only for resources included in this package.
        registry = {key: value for key, value in registry.items() if ROOT / value in selected}
        for source in sorted(selected):
            copy(source, source.relative_to(ROOT))
        (stage / 'data').mkdir(exist_ok=True)
        (stage / 'data/resources.registry').write_text(json.dumps(registry, ensure_ascii=False, indent=2), encoding='utf-8')
        (stage / 'data/characters').mkdir(exist_ok=True)

        # Include available upstream dependency license files, with no source code.
        registry_roots = list((Path.home() / '.cargo/registry/src').glob('*'))
        for package in tomllib.loads((ROOT / 'Cargo.lock').read_text(encoding='utf-8'))['package']:
            if not package.get('source', '').startswith('registry+'):
                continue
            ident = f"{package['name']}-{package['version']}"
            for registry_root in registry_roots:
                directory = registry_root / ident
                if directory.is_dir():
                    for license_file in directory.iterdir():
                        if license_file.is_file() and license_file.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE')):
                            copy(license_file, Path('licenses/rust') / ident / license_file.name)
                    break

        (stage / 'Play.cmd').write_bytes(b'@echo off\r\ncd /d "%~dp0"\r\ntsb.exe\r\nif errorlevel 1 pause\r\n')
        diagnostics = stage / 'diagnostics'
        diagnostics.mkdir()
        (diagnostics / 'Check.ps1').write_text(LAUNCHER, encoding='utf-8-sig')
        for filename, mode in [('Check-EOS', 'Check'), ('Check-Relay-Host', 'RelayHost'), ('Check-Relay-Join', 'RelayJoin')]:
            (diagnostics / f'{filename}.cmd').write_bytes((f'@echo off\r\npowershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Check.ps1" -Mode {mode}\r\n').encode('ascii'))
        (stage / 'README-KO.txt').write_text(README, encoding='utf-8-sig')
        (stage / 'licenses/Microsoft-runtime.txt').write_text(
            'Microsoft Visual C++ x64 runtime and XAudio2 redistributable DLLs.\n'
            'https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files\n'
            'https://learn.microsoft.com/windows/win32/xaudio2/xaudio2-redistributable\n', encoding='utf-8')
        files = []
        for path in sorted(stage.rglob('*')):
            if path.is_file():
                files.append({'path': path.relative_to(stage).as_posix(), 'bytes': path.stat().st_size, 'sha256': digest(path)})
        manifest = {'name': args.name, 'platform': 'Windows 10/11 x64', 'cargo_profile': args.profile,
                    'eos_config_included': True, 'source_code_included': False, 'files': files}
        (stage / 'package-manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding='utf-8')
        print(f'Staged {len(files)} files, {sum(f["bytes"] for f in files) / 1048576:.1f} MiB: {stage}')

    if not args.stage_only:
        if archive.exists():
            raise SystemExit('ZIP already exists; it will not be overwritten')
        manifest = json.loads((stage / 'package-manifest.json').read_text(encoding='utf-8'))
        for item in manifest['files']:
            if digest(stage / item['path']) != item['sha256']:
                raise SystemExit(f'Packaged file changed after staging: {item["path"]}')
        pending = archive.with_suffix('.zip.part')
        with zipfile.ZipFile(pending, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=6,
                             strict_timestamps=False) as z:
            # Explicit manifest list excludes any QA logs/caches created in stage.
            for relative in [item['path'] for item in manifest['files']] + ['package-manifest.json']:
                z.write(stage / relative, f'{args.name}/{relative}')
        with zipfile.ZipFile(pending) as z:
            if z.testzip() is not None:
                raise SystemExit('ZIP CRC validation failed')
            if len(z.infolist()) != len(manifest['files']) + 1:
                raise SystemExit('ZIP file count validation failed')
        pending.rename(archive)
        checksum = digest(archive)
        (output / f'{args.name}.zip.sha256').write_text(f'{checksum}  {archive.name}\n', encoding='ascii')
        print(f'ZIP ready: {archive} ({archive.stat().st_size / 1048576:.1f} MiB)')
        print(f'SHA256 {checksum}')


if __name__ == '__main__':
    main()
