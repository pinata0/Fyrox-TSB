# Windows 단일 EXE 패키징

## v0.1 공개본 (2026-11-21)

v0.1은 최신 소스를 릴리스 빌드한 `release/v0.1/TSB.exe`이며, 루트 배포 경로 `release/TSB.exe`도 같은 파일을 사용합니다. [GitHub 릴리즈](https://github.com/pinata0/Fyrox-TSB/releases/tag/v0.1)에 단일 EXE와 선택형 SHA-256 텍스트를 게시합니다. 개발 시작일은 2026-11-12, 게시일은 2026-11-21입니다.

- 2026-11-21 마요 1인칭 팔 핫픽스이며 버전 표기는 v0.1로 유지합니다.
- 크기: 339,195,143바이트 / 내부 파일: 1,766개.
- SHA-256: `9dcd2536f2a37f4349331a446c47a87985cf15d2d2cc7c9150e5e0847ffc5cc7`.
- 빌드 stage: `artifacts/releases/TSB-v0-1-20261121-mayo-hotfix`.
- 핫픽스 검증 기록: `artifacts/mayo-hotfix-v0.1/validation.json` (추출·EOS·마요 30개 화면·실행 후 무결성).
- 초기 공개본 검증 기록: `artifacts/v0.1-validation/results.json`, `artifacts/v0.1-headless-full.log`.
- 검증 결과와 알려진 제한은 [v0.1 릴리즈 노트](releases/v0.1.md)에 정리했습니다. 아래 내용은 기존 패키징 방식과 이전 저폴리곤 빌드의 작업 기록입니다.

## 패키징 방식과 이전 빌드 기록

최신 배포본(2026-11-21): 프로젝트 루트의 `release/TSB.exe`.
최신 개발 내용, 성능 최적화, 저폴리곤 캐릭터·무기와 표정 데이터를 포함합니다.
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
python scripts/package_portable.py --include-eos-config --profile release --name TSB-single-exe-20261121-lowpoly --stage-only
python scripts/package_single_exe.py --stage artifacts/releases/TSB-single-exe-20261121-lowpoly --output artifacts/single-exe/20261121-lowpoly/TSB.exe
```

실행기 디렉터리에서 빌드해야 `.cargo/config.toml`의 정적 CRT 설정이 적용됩니다.
위 stage와 후보 EXE 경로는 이 빌드에서 사용한 경로이므로, 다음 빌드에는 새 이름을 지정하세요.
후보 EXE를 별도 작업 폴더에서 `--verify-only`, `--check-eos --offline`,
`--solo --auto-start --lan --smoke-test`로 확인한 뒤 `release/TSB.exe`를 교체합니다.
`release/TSB.zip`과 Linux 배포본은 이번 Windows 단일 EXE 재빌드 대상에 포함되지 않았습니다.
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

## 2026-11-21 빌드 검증

- 단일 EXE: 339,145,244바이트, 내부 파일 1,766개, 추출 크기 473,440,253바이트.
- 최신 게임/진단 실행파일과 저폴리곤 런타임 에셋 231개를 원본 SHA-256과 대조했습니다.
- 별도 작업 폴더에서 최초 추출, 파일별 무결성 검사, EOS DLL 오프라인 초기화,
  연습 모드 구동과 정상 종료, 아야 E/Q의 10개 GPU 검증 장면을 확인했습니다.
- 15개 캐릭터와 각 23개 표정 채널이 정상 로드됐으며, 게임 종료 후 캐시 재검사도 통과했습니다.
- 기존 `--smoke-test`는 5개 화면을 캡처했으나 종합 완료 조건에서 실패했습니다.
  단일 EXE 재실행과 추출된 게임 직접 실행에서도 실패했고, 저장소의 원본 실행파일에서는
  통과했습니다. 원인은 확정하지 않았으며 전체 회귀 테스트 통과를 의미하지 않습니다. 상세 결과는
  `artifacts/single-exe/20261121-lowpoly/validation.json`에 보관합니다.
- 이번 검증은 로컬 구동과 패키지 구성 확인이며, 다른 PC와의 온라인 접속은 검사하지 않았습니다.

최종 파일의 SHA-256은 `release/TSB.exe.sha256`에 기록합니다.
