# Linux x64 빌드

배포 파일: `release/TSB-Linux-x64.tar.gz`. Windows용 `TSB.exe`와 별개인 Linux ELF 실행 파일과 동일한 게임 에셋, Linux EOS 런타임, 게임용 EOS 설정을 포함합니다. Intel/AMD x86_64 대상이며 ARM 및 macOS용은 아닙니다.

## 실행

```sh
tar -xzf TSB-Linux-x64.tar.gz
cd TSB-Linux-x64
./Play.sh
```

`Play.sh`는 자기 폴더로 이동한 뒤 게임을 실행합니다. 압축을 푼 폴더는 게임이 로그와 리소스 캐시를 쓸 수 있어야 합니다. 압축 프로그램이 실행 권한을 보존하지 않았다면 다음 명령을 한 번 실행합니다.

```sh
chmod +x Play.sh tsb tsb-server tsb-eos-check tsb-eos-peer-check
```

Ubuntu 22.04 / glibc 2.35를 기준으로 빌드합니다. 데스크톱 그래픽 드라이버와 OpenGL, ALSA, X11/Wayland 및 xkbcommon 런타임이 필요합니다. Rust나 개발 도구는 필요 없습니다. Wayland 데스크톱에서 X11 경로를 사용하려면 XWayland가 설치되어 있어야 합니다. `libEOSSDK-Linux-Shipping.so`는 실행 파일 옆에서 `$ORIGIN`으로 찾습니다.

`libxkbcommon-x11.so`를 찾을 수 없다는 오류가 나면 Ubuntu/Debian에서 `sudo apt install libxkbcommon-x11-0`으로 해당 데스크톱 라이브러리를 설치합니다.

기존 Windows판과 같은 EOS 제품 설정을 사용합니다. 방 만들기/코드 참가, 오프라인 연습과 직접 UDP 기능을 유지합니다. Windows/Linux 간 별도 PC 멀티플레이와 실제 사용자 GPU·마이크 동작은 별도 확인이 필요합니다.

## Windows에서 재빌드

Docker Desktop의 Linux 엔진과 EOS SDK가 필요합니다. SDK는 프로젝트 밖 `../tools/EOS-SDK-*/SDK`에서 자동으로 찾거나 `-SdkPath`로 지정합니다.

```powershell
.\scripts\Build-Linux.ps1
python scripts/package_linux.py --include-eos-config
```

스크립트는 Ubuntu 22.04와 Rust 1.95.0 빌드 이미지를 만들고 `Cargo.lock`을 고정해 릴리스 빌드, EOS 포함 헤드리스 테스트, SDK 초기화 및 동적 라이브러리 검사를 실행합니다. 최초 빌드는 이미지와 의존성을 다운로드합니다. 이후 `-SkipImageBuild`로 이미지 빌드를 생략할 수 있습니다. 컴파일 캐시는 `tsb-linux-target`, Cargo 레지스트리는 `tsb-linux-registry` Docker 볼륨에 보관합니다.

실행 파일은 `target/linux-x64/release`, 검증 로그는 `artifacts/linux-build`에 생성합니다. 기존 Windows 출력 폴더는 사용하지 않습니다. 패키지는 실행 권한을 보존하며 모든 파일 SHA-256을 기록하고, 압축 후 다시 읽어 검증합니다. 기존 배포 파일을 덮어쓰지 않으므로 재패키징할 때는 `--name TSB-Linux-x64-v2`처럼 다른 이름을 지정합니다.

## Linux에서 직접 빌드

`scripts/linux/Dockerfile`의 개발 라이브러리와 Rust 1.95.0을 설치한 환경에서:

```sh
export EOS_SDK_DIR=/path/to/EOS-SDK/SDK
cargo build --locked --release --bins
cargo test --locked --release --no-default-features --features eos
./target/release/tsb-eos-check --offline
./target/release/tsb --solo
```

SDK 없이 직접 UDP와 오프라인 연습만 빌드하려면 `cargo build --locked --release --no-default-features --features desktop --bin tsb`를 사용합니다.

## 2026-11-17 검증 결과

- Linux x64 릴리스 빌드 성공, EOS 포함 자동 테스트 **266개 통과**.
- Windows EOS 회귀 테스트 3개 통과.
- 배포 압축의 1,326개 파일 해시, Linux 실행 권한, ELF x86_64 형식과 `$ORIGIN` 런타임 경로 확인. 실행 파일이 요구하는 최신 glibc 심벌은 `GLIBC_2.35`입니다.
- 압축 해제본에서 `Play.sh --help`, EOS 1.19.2.1 초기화 및 라이브러리 로딩 통과.
- Xvfb/Mesa llvmpipe에서 제작사 시작 화면을 렌더링하고 캡처 확인.
- **14초 플레이 검사는 통과하지 못했습니다.** 소프트웨어 렌더링의 첫 셰이더 준비/프레임 처리 시간이 길었으며, 프레임 기준 시작 화면 검사도 240초 안에 전체 단계를 마치지 못했습니다. 실제 GPU의 플레이 성능·마이크·Windows와의 두 PC 멀티플레이는 미검증입니다.

빌드와 테스트 로그는 `artifacts/linux-build/`, 시작 화면 캡처는 `artifacts/linux-build/package-check/linux-startup.png`에 있습니다. `scripts/linux/verify-package.sh`는 전체 화면/플레이 검사를 시도하며 위 가상 그래픽 환경의 제한 때문에 실패할 수 있습니다.
