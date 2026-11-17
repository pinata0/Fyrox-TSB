"""Pack a verified portable stage into a native, self-extracting Windows EXE.

The launcher needs no Rust, .NET, PowerShell, installer or administrator rights.
It caches this build under %LOCALAPPDATA%/TSB/SingleFile and runs the game there.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import zlib

from package_portable import ROOT, digest

MAGIC = b'TSB-ONEFILE-v1\0\0'
README = '''Trickcal : Shooting Balddagu — Windows x64 단일 EXE 테스트판

TSB-Multiplayer-Portable.exe 하나만 다른 PC에 복사한 뒤 더블클릭하세요.
최초 실행 시 필요한 에셋과 DLL을 자동으로 풀고 게임을 시작합니다.
흰 배경의 제작사 크레딧 후 로그인 / 회원가입 / 게스트로 로그인하기를 선택하세요.
로그인은 게임 안에서 이메일과 비밀번호를 입력하며 게스트는 바로 홈으로 이동합니다.
홈 → 프로필 → 프로필 사진 변경에서 파일 선택·미리보기·저장·삭제가 가능합니다.
회원가입과 이메일 확인은 DOTG 웹사이트에서 진행합니다.
별도 설치, Rust, Visual Studio, EOS SDK 설치나 관리자 권한은 필요 없습니다.
Windows 10/11 64비트와 OpenGL 지원 그래픽 드라이버가 필요합니다.
캐시/로그 위치: %LOCALAPPDATA%\\TSB\\SingleFile\\빌드해시\\artifacts\\logs

멀티플레이 (서로 다른 PC에서 동일한 EXE 사용)
1. 호스트: 홈 → 플레이 → 방 만들기 → 로비의 ‘코드 복사하기’ 버튼 클릭.
2. 참가자: 홈 → 플레이 → 방 참여하기 → 방 코드 입력란에서 Ctrl+V → 참여하기.
3. 호스트가 게임 시작을 누른 뒤 참가자는 게임 참가를 누릅니다.
웹 계정이나 공유기 포트포워딩 없이 전체 eos:... 코드로 참가할 수 있습니다.
게임용 EOS 설정을 포함했으며, 기기 계정은 각 PC에서 별도로 생성됩니다.
같은 PC에서 두 창을 테스트할 때는 --host / --join의 LAN 모드를 사용하세요.

선택적 명령줄 진단 (결과는 위 로그 폴더의 launcher-프로세스번호.log)
TSB-Multiplayer-Portable.exe --check-eos
TSB-Multiplayer-Portable.exe --check-relay-host --seconds 300
TSB-Multiplayer-Portable.exe --check-relay-join eos:호스트ID:소켓 --seconds 300
TSB-Multiplayer-Portable.exe --verify-only

조작: WASD 이동 / 마우스 시점 / 왼쪽 클릭 공격 / E 스킬 / Q 궁극기 /
H 캐릭터 선택 / Enter 채팅 / M 마이크(기본 음소거) / Esc 메뉴 / F10 종료.
licenses 폴더에 의존성 라이선스와 EOS 고지를 포함했습니다.
'''


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', type=Path, required=True)
    parser.add_argument('--launcher', type=Path, default=ROOT / 'scripts/onefile-launcher/target/x86_64-pc-windows-msvc/release/tsb-onefile.exe')
    parser.add_argument('--output', type=Path, default=ROOT / 'release/TSB-Multiplayer-Portable.exe')
    args = parser.parse_args()
    source_manifest = json.loads((args.stage / 'package-manifest.json').read_text(encoding='utf-8'))
    if source_manifest['cargo_profile'] != 'release':
        raise SystemExit('A release-profile stage is required')
    entries = []
    for entry in source_manifest['files']:
        if entry['path'].endswith(('.cmd', '.ps1')) or entry['path'] == 'README-KO.txt':
            continue
        if digest(args.stage / entry['path']) != entry['sha256']:
            raise SystemExit(f'Stage checksum mismatch: {entry["path"]}')
        entries.append(entry)
    readme = README.replace('TSB-Multiplayer-Portable.exe', args.output.name).encode('utf-8-sig')
    entries.append({'path': 'README-KO.txt', 'bytes': len(readme), 'sha256': hashlib.sha256(readme).hexdigest()})
    manifest = json.dumps(entries, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    pending = args.output.with_suffix('.exe.part')
    if args.output.exists() or pending.exists():
        raise SystemExit('Output already exists; use another --output name')
    payload_hash = hashlib.sha256()
    payload_size = 0
    compressor = zlib.compressobj(level=6)
    with pending.open('xb') as output, args.launcher.open('rb') as launcher:
        shutil.copyfileobj(launcher, output)

        def emit(data: bytes) -> None:
            nonlocal payload_size
            output.write(data)
            payload_hash.update(data)
            payload_size += len(data)

        emit(compressor.compress(struct.pack('<I', len(manifest)) + manifest))
        for entry in entries:
            if entry['path'] == 'README-KO.txt':
                emit(compressor.compress(readme))
            else:
                with (args.stage / entry['path']).open('rb') as source:
                    while chunk := source.read(1024 * 1024):
                        emit(compressor.compress(chunk))
        emit(compressor.flush())
        output.write(MAGIC + struct.pack('<Q', payload_size) + payload_hash.digest())
    pending.rename(args.output)
    report = {
        'file': str(args.output), 'bytes': args.output.stat().st_size,
        'sha256': digest(args.output), 'payload_sha256': payload_hash.hexdigest(),
        'cargo_profile': 'release', 'file_count': len(entries),
        'extracted_bytes': sum(entry['bytes'] for entry in entries),
        'eos_config_included': True,
    }
    report_path = ROOT / 'artifacts/single-exe/package-report.json'
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
