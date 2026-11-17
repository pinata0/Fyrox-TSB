# EOS SDK 연결과 설정 점검

현재 Windows x64/MSVC용 EOS C SDK를 `eos` Cargo 기능으로 연결합니다. SDK 원본은 프로젝트 밖에 보관하며, 필요한 DLL만 Cargo 출력 폴더에 복사합니다. 기본 빌드는 EOS를 포함합니다. SDK 없이 UDP만 빌드하려면 `cargo build --no-default-features --features desktop --bin tsb`를 사용합니다.

## SDK 위치

이 PC에서 확인한 SDK는 다음 폴더입니다.

```text
C:\Users\user\projects\tools\EOS-SDK-58105819-Release-v1.19.2.1\SDK
```

`../tools` 아래에 SDK가 하나만 있으면 자동으로 찾습니다. 다른 위치에 두거나 여러 버전을 보관할 때는 `EOS_SDK_DIR` 환경 변수 또는 `scripts/Build-Eos.ps1 -SdkPath <경로>`를 사용합니다. 경로는 `Include`, `Lib`, `Bin`이 있는 SDK 폴더 또는 그 바로 위 압축 해제 폴더입니다. 경로·키를 Cargo 소스에 하드코딩하지 않습니다.

```powershell
# 게임과 점검 도구 빌드 + 자격증명 없이 SDK 초기화 검사
.\scripts\Build-Eos.ps1 -Check

# 그래픽 게임을 빌드하지 않고 점검 도구만 실행
cargo run --offline --no-default-features --features eos --bin tsb-eos-check -- --offline
```

PowerShell 실행 정책이 이 로컬 스크립트를 막는 경우 `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Build-Eos.ps1 -Check`로 해당 실행에만 허용할 수 있습니다. 시스템 전체 실행 정책을 변경할 필요는 없습니다.

`tsb.exe`를 다른 폴더에 배포할 때 EOS를 사용하는 빌드에는 `EOSSDK-Win64-Shipping.dll`도 함께 배치해야 합니다. SDK 원본 전체나 `eos.local.json`을 배포/커밋하지 않습니다. SDK 라이선스 및 ThirdPartyNotices도 출시 전에 검토합니다.

## Developer Portal에서 값 찾기

[Epic Developer Portal](https://dev.epicgames.com/portal/) → 게임 제품 → **Product Settings**에서 찾습니다.

| JSON 필드 | 포털의 값 |
|---|---|
| `product_id` | Product ID |
| `sandbox_id` | 사용할 Sandbox의 Sandbox ID |
| `deployment_id` | 해당 Sandbox에 속한 Deployment ID |
| `client_id` | Clients에서 만든 게임용 Client ID |
| `client_secret` | 같은 게임용 Client의 Client Secret |

`SDK Download & Credentials`에서 제품 자격증명을 확인하고, `Clients`에서 게임용 클라이언트를 관리합니다. `TSB-P2P-Policy`는 정책 이름이며 Client ID가 아닙니다. 정책만 만들었다면 `Clients → Add new client`에서 `TSB-Game-Client`를 만들고 `TSB-P2P-Policy`를 선택합니다. 정책 유형은 `Peer2Peer`, `User required`는 활성화합니다. Build Patch Tool 자격증명을 넣지 않습니다.

프로젝트 루트의 `eos.local.json`에 다섯 값을 입력합니다. 예시 형식은 `eos.example.json`입니다. 키는 빈 따옴표 안에 붙여 넣고, JSON 필드 이름은 바꾸지 않습니다. `eos.local.json`은 Git에서 제외하며 진단 메시지에 자격증명 값을 출력하지 않습니다.

## 온라인 점검

```powershell
.\scripts\Build-Eos.ps1 -Check -ConfigFile .\eos.local.json

# 이미 빌드했다면
.\target\debug\tsb-eos-check.exe --config .\eos.local.json
```

온라인 점검은 해당 EOS 프로젝트에 Device ID 사용자로 로그인하며, 처음이면 EOS 기기 사용자 ID를 생성합니다. Epic 계정 로그인 화면은 사용하지 않습니다. 점검 출력은 SDK 버전, 로그인 결과, Product User ID, NAT 유형입니다. 기기 사용자는 Supabase 계정과 별개이며, DotG 프로필 인증이나 게임 입장 권한을 증명하지 않습니다.

점검 도구는 SDK를 초기화하고 플랫폼을 틱한 뒤 종료 시 해제합니다. 로그인은 최대 30초, NAT 조회는 최대 20초 대기합니다. 잘못된 자격증명·정책·네트워크 설정은 실패로 보고하고, SDK 초기화 성공을 온라인 로그인 성공으로 간주하지 않습니다.

`--force-relay`는 플랫폼에 강제 중계 정책을 설정합니다. **두 PC 사이의 실제 중계 연결을 검증하는 명령은 아닙니다.** 기본값은 직접 연결 시도 후 중계를 허용하는 정책입니다.

## 게임에서 사용하기

기본 온라인 방은 **EOS P2P**로 게임 입력·스냅샷·채팅·보이스를 전송합니다. 호스트와 참가자가 직접 연결을 시도하고, 불가능하면 Epic 중계를 사용합니다. 두 PC 모두 같은 Product/Sandbox/Deployment의 게임용 설정과 `EOSSDK-Win64-Shipping.dll`이 필요합니다. EOS Device ID는 PC별 전송 사용자이며, 같은 PC의 두 프로세스는 같은 ID라 EOS 상호 접속 테스트에 사용할 수 없습니다. 같은 PC 테스트는 `--lan`을 사용하세요.

웹 방 목록과 연결된 실행: 웹사이트에 `20261113110000_add_game_room_directory.sql` 및 `20261113120000_add_eos_p2p_rooms.sql`과 API 변경을 적용합니다. 기본 웹 주소는 `https://www.dotgsince2023.co.kr/play`이며 다른 웹사이트를 사용할 때만 `DOTG_PORTAL_URL`을 설정합니다. 홈 → 플레이 → 웹 계정 연결 → 방 만들기에서 이름만 입력하면 EOS 주소가 자동으로 등록됩니다. 참가자는 목록이나 8자리 방 코드로 참여합니다. 웹 계정과 EOS ID를 영구적으로 연결하는 기능은 아니며, 계정 권한은 서명된 참가권으로 검사합니다.

배포 ZIP은 `Play.cmd` 하나로 실행합니다. 호스트는 게임 홈 → 플레이 → 방 만들기를 누르면 웹 계정 없이 EOS 방을 만듭니다. EOS 준비 후 로비 상단에 전체 방 코드가 표시되며 **코드 복사하기** 버튼으로 복사합니다. 참가자는 같은 게임의 플레이 → 방 참여하기 → 방 코드 입력란에서 **Ctrl+V** → 참여하기를 누릅니다. 혼자 연습은 홈의 전투훈련에서 시작합니다. 웹 계정을 연결한 상태에서 만드는 방은 기존 웹 목록 등록과 참가권 검사를 사용합니다.

개발용 명령줄로도 테스트할 수 있습니다:

```powershell
# 호스트: 로비에서 준비가 끝나면 ‘코드 복사하기’ 버튼 클릭
.\target\debug\tsb.exe --eos-host
# 참가자: 홈 → 플레이 → 방 참여하기 → 방 코드 입력란에서 Ctrl+V 또는 다음 명령
.\target\debug\tsb.exe --join 'eos:호스트ID:방소켓'
# 중계만 사용하도록 진단
.\target\debug\tsb.exe --eos-host --force-relay
```

`--eos-config <파일>`로 기본 `eos.local.json` 대신 다른 설정 파일을 선택할 수 있습니다. 기존 `--host IP:포트`, `--join IP:포트`, `--lan`은 직접 UDP입니다. EOS 실패를 UDP로 자동 우회하지 않습니다. 설정 실패는 화면에 표시하며, 설정 수정 후 게임을 다시 실행합니다. 방을 나갔다 다시 만들어도 EOS SDK는 한 번 초기화한 상태를 유지하며 새 소켓 비밀을 사용합니다.

## 두 PC 접속 점검

그래픽 없는 점검 도구도 실제 `Server`/`Client`와 게임 패킷을 사용합니다. 로그인만으로 성공 처리하지 않고 스냅샷 분할 재조립, 양방향 채팅 및 EOS가 보고한 실제 연결 유형을 확인합니다.

```powershell
# 서로 다른 네트워크의 호스트 PC
.\target\debug\tsb-eos-peer-check.exe --host --force-relay --seconds 180
# 위에서 출력한 HOST_READY 코드를 다른 PC에 전달
.\target\debug\tsb-eos-peer-check.exe --join 'eos:호스트ID:방소켓' --force-relay --seconds 180
```

양쪽 모두 `PASS`와 `EOS Relay`를 확인합니다. `--force-relay` 없이 다시 실행하면 직접 연결 또는 자동 중계를 검사합니다. 이어서 실제 게임에서 방 나가기/재생성, 맵 선택, 캐릭터·이동·채팅·팀 보이스를 확인하세요. 팀 보이스는 같은 팀 참가자가 필요합니다. `--ready-only --host`는 로그인과 P2P 수신 준비만 확인하며 **두 PC 접속 검증이 아닙니다**.

전송은 별도 EOS 스레드에서 틱하고, 1170바이트를 넘는 기존 게임 패킷은 두 조각으로 나눕니다. 분할 패킷·송수신 큐는 크기와 수명을 제한하며, 게임의 입력 순번·인증·선택/채팅 재전송을 유지합니다. EOS 로그인 만료 알림은 Device ID 재로그인으로 갱신합니다.

이 작업에서 로컬 회귀 테스트, EOS 로그인, NAT 조회(strict), 강제 중계 정책 및 P2P 수신 소켓 준비를 확인했습니다. 서로 다른 네트워크의 두 PC 간 실제 Direct/Relay 접속은 두 번째 PC에서 위 점검을 실행해야 확정할 수 있습니다.

참고: [EOS 클라이언트 설정](https://dev.epicgames.com/docs/epic-online-services/eos-fundamentals/client-and-client-policy/client-policy-guide), [P2P 소개](https://dev.epicgames.com/docs/epic-online-services/multiplayer/nat-p2p-interface/p2p-guide/p2p-intro)
