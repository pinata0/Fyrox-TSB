# Windows 단일 EXE 패키징

최신 배포본(2026-11-17): 프로젝트 루트의 `release/TSB-Multiplayer-ErpinMotions.exe`.
패키징 스크립트의 기본 출력 이름은 `TSB-Multiplayer-Portable.exe`이며, 배포본 이름은 `--output`으로 지정한다.
기존 게임 실행 파일, EOS 진단 도구, EOS/VC++/XAudio2 DLL, 런타임 에셋,
게임 클라이언트용 EOS 설정, 라이선스를 압축해 하나의 네이티브 실행 파일에 담습니다.
공유 대상은 이 EXE 하나입니다. EOS 기기 계정이나 웹 로그인 세션은 포함하지 않습니다.

## 빌드

프로젝트 루트에서 실행합니다. 기존 경로를 덮어쓰지 않으므로 재빌드 시 새 stage 이름과
`--output` 경로를 지정하거나 기존 배포 파일을 다른 이름으로 보관하세요.

```powershell
cargo build --locked --offline --release --bin tsb --bin tsb-eos-check --bin tsb-eos-peer-check
Push-Location scripts/onefile-launcher
cargo build --locked --offline --release
Pop-Location
python scripts/package_portable.py --include-eos-config --profile release --name TSB-single-exe-release --stage-only
python scripts/package_single_exe.py --stage artifacts/releases/TSB-single-exe-release
```

실행기 디렉터리에서 빌드해야 `.cargo/config.toml`의 정적 CRT 설정이 적용됩니다.
패키징은 stage 파일의 SHA-256을 검사하고, 압축 페이로드의 SHA-256과 길이를 EXE 끝에
덧붙입니다. 빌드 결과와 최종 SHA-256은 `artifacts/single-exe/package-report.json`에 남깁니다.

## 실행

관리자 권한 없이 `%LOCALAPPDATA%\TSB\SingleFile\페이로드해시`에 자동으로 풉니다.
실행기는 운영체제 DLL만 필요하며 .NET, PowerShell, Rust 설치를 요구하지 않습니다.
첫 실행에서 파일별 SHA-256을 검사한 뒤 완성된 캐시를 공개합니다. 이후 실행은 캐시를
재사용합니다. 동시에 실행되거나 캐시가 손상된 경우 별도의 완성된 복사본을 사용하며,
실행 중인 파일을 덮어쓰지 않습니다. 캐시는 종료 후에도 남아 다음 실행을 빠르게 합니다.
엔진이 실행 후 갱신하는 `data/resources.registry`는 최초 추출 때만 원본 해시를 검사합니다.

게임은 캐시 폴더를 작업 디렉터리로 사용하므로 EXE를 어디에 복사해도 같은 에셋을 찾습니다.
`--eos-config`, `--online-session`, 진단용 `--config`와 `--code-file`의 상대 경로는
원래 실행 위치를 기준으로 해석합니다. 게임 창만 표시하며 콘솔 출력은 실행별 로그에 저장합니다.

검증 옵션:

- `--extract-only`: 파일을 준비한 뒤 게임을 시작하지 않고 종료.
- `--verify-only`: 파일별 해시까지 검사하고 필요하면 새 복사본을 추출.
- `--check-eos [--offline]`: 포함된 EOS SDK/로그인 진단.
- `--check-relay-host`, `--check-relay-join CODE`: 포함된 실제 전송 진단.
- 기존 게임 인자 (`--solo`, `--host`, `--join`, `--smoke-test` 등)는 그대로 전달.
- `TSB_CACHE_DIR` 환경 변수: QA용 캐시 위치 변경. 일반 사용자는 설정할 필요 없음.

한 PC의 두 프로세스는 EOS Device ID를 공유하므로 EOS 중계의 실제 양방향 접속 검증에는
서로 다른 PC가 필요합니다. 로컬 UDP 회귀 테스트나 EOS 로그인 성공만으로 이 검증을
대체하지 않습니다.
