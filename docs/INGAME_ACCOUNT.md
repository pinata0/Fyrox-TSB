# 인게임 소셜·프로필

홈의 소셜·프로필 메뉴와 오른쪽 상단 버튼은 `presentation/account.rs`의 네이티브 화면을 엽니다. 버튼 클릭만으로 브라우저를 열지 않습니다. 게스트도 같은 화면을 열며, 로그인 버튼을 선택할 때 기존 로그인/회원가입 절차를 시작합니다.

## 기능

- 친구 목록, 받은 요청, 보낸 요청, 친구 찾기, 차단 관리, 방 초대, 내 프로필 탭.
- 사용자 ID/닉네임 검색, 요청·수락·거절·취소·삭제, 차단·해제.
- 내 방으로 초대, 받은 초대 거절 또는 즉시 게임 참여. 이미 방에 있으면 먼저 방에서 나가도록 안내합니다.
- 이름(본인에게만 표시), 닉네임, 사용자 ID, 소개 240자, 기본 프로필 색상 저장. 저장 후 홈 카드도 갱신합니다.
- 이메일·비밀번호 로그인 화면. 비밀번호와 IME 입력을 마스킹하고 제출·취소·게스트 전환 시 지웁니다. 로그인·재로그인은 브라우저를 열지 않습니다. 회원가입·이메일 확인은 기존 웹 절차를 사용합니다. 2단계 인증이 필요한 계정은 현재 네이티브 로그인에서 지원하지 않으며 우회하지 않습니다.
- 프로필 사진 파일 선택창, 미리보기, 명시적인 저장, 기본 사진으로 복원. JPG·PNG·WebP / 2MB·1,600만 화소 이하를 지원합니다. 사진 선택과 업로드가 모두 게임 안에서 이루어집니다.
- 한글 IME, UTF-8 커서 편집, Ctrl+A/V, Tab/Shift+Tab, Enter, F5, 페이지 이동, Esc 복귀.
- 삭제/차단 확인, 미저장 프로필을 버리고 홈으로 돌아갈 때 확인. 새로고침은 작성 중 내용을 덮어쓰지 않습니다.
- 로딩·빈 목록·통신 오류·로그인 만료 상태. 계정이 바뀌면 이전 계정의 목록·작성 내용을 지웁니다. 소셜은 20초 간격으로 갱신합니다.

## 서버 적용

기존 계정 연결 토큰을 `room_directory` 작업 스레드에서 `/api/game/account`에 전달합니다. 인증 정보는 파일에 저장하거나 로그로 출력하지 않습니다. 렌더 스레드는 HTTP를 기다리지 않으며, 기존 방 목록·프로필 동기화와 같은 계정 세션을 사용합니다.

`/api/game/auth`는 Supabase 이메일·비밀번호 인증을 수행하고 계정 소유의 게임 기기 토큰을 발급합니다. 접근·갱신 토큰은 작업 스레드의 메모리에서만 보관합니다. 사진 변경에 필요한 접근 토큰은 만료 전에 갱신하며, 로그아웃과 종료 시 해당 기기와 인증 세션을 해제합니다. 파일을 선택하는 것만으로 업로드하지 않으며 서버는 저장 요청의 사진을 512px WebP로 변환하고 메타데이터를 제거합니다. 사진 API는 사용자 JWT와 같은 사용자에게 발급된 유효한 게임 기기 토큰을 모두 요구합니다.

함께 수정된 웹 소스: `C:/Users/user/projects/DotGwebsite/dotg-web`.

1. `20261115090000_add_native_player_profile.sql` 다음에 `20261115100000_add_native_social.sql` 적용.
2. `/api/game/account`와 네이티브 입력 검증, RPC 타입을 포함한 웹 앱 배포.
3. 새 게임 실행 파일 사용.

2026-11-15 소셜·프로필 운영 반영 완료: [PR #15](https://github.com/dotgofgist/DotGwebsite/pull/15), 커밋 `5eaa8a60b7aef1857493aafe07acdc41a0b0e19d`, [운영 배포 실행](https://github.com/dotgofgist/DotGwebsite/actions/runs/43622019470). 운영 DB에 `20261115100000`을 적용했고 `https://www.dotgsince2023.co.kr/api/game/account`가 실제 DB에 연결됩니다. 게임의 기본 서버 주소이므로 별도 환경 변수 설정은 필요하지 않습니다.

네이티브 로그인·사진 API 운영 반영 완료: [PR #16](https://github.com/dotgofgist/DotGwebsite/pull/16), 커밋 `313f2e2a48f5678d26d4132b204b984bfcbfc8c0`, [운영 배포 실행](https://github.com/dotgofgist/DotGwebsite/actions/runs/43627752579). 운영 DB에 `20261115110000_add_native_login.sql`을 적용했습니다.

## 검증

- `cargo test --locked`: 게임·네트워크·UI 회귀 테스트. 로컬 HTTP/UDP를 허용하는 환경에서 실행합니다.
- `cargo clippy --locked --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
- `cargo run --locked --bin tsb -- --solo --account-review`: 실제 Fyrox 렌더러로 프로필·소셜·확인·게스트·오류·사진 미리보기·파일 선택창 16장 캡처. 테스트용 데이터만 사용하고 계정 API를 호출하지 않습니다. 1280×800, 960×640, 1920×800을 검증합니다.
- `--startup-review`: 로고·계정 선택·네이티브 로그인·마스킹·오류·회원가입 대기 9장 캡처. `artifacts/ingame-native-login/*-review-final.log`에 결과를 남깁니다.
- 로컬 HTTP 회귀 검사에서 네이티브 로그인·토큰 갱신·사진 업로드·삭제·로그아웃 순서와 각 자격 증명의 전달 대상을 검증합니다.
- `artifacts/preview-account-*-menu.png`, `artifacts/ingame-account/review-final.log`.
- 웹 Node/PGlite 테스트에서 기기 토큰 소유권·미승인/만료 거부·친구 수락 권한·차단·프로필 동기화·초대 정원·참가권 서명과 기존 웹 동작을 확인합니다.

운영 검증 결과:

- 실제 도메인의 공개 페이지·계정 보호·방 목록·계정 API 검사를 통과했습니다. 누락 토큰 400, 브라우저 Origin 403, 잘못되거나 미승인된 기기 토큰 401과 no-store를 확인했습니다.
- 운영 DB에서 임시 프로필을 저장하고 다시 조회했으며 홈 정보 동기화와 만료 토큰 거부도 확인했습니다. 한 트랜잭션 안에서 검사한 뒤 모두 롤백했고 임시 계정이 0개 남았음을 확인했습니다. SQL: `artifacts/ingame-account/production-native-rollback-check.sql`.
- GitHub 정적 검사·프로덕션 빌드·DB 검사(137개 SQL 검증 및 타입/권한 검사) 통과. E2E 기능 검사 63개 통과. 변경하지 않은 웹페이지의 색상 대비 검사 20개는 실패했으며 PR에 기록했습니다. 미리보기 토큰과 Dependency graph 설정 문제도 별도로 남아 있습니다.
- 실제 사용자 계정으로 게임 창에서 저장하거나 서로 다른 PC에서 초대 참여하는 전체 흐름은 이번 운영 검증에서 수행하지 않았습니다.

로그인·사진 추가 검증:

- 게임 전체 테스트·Clippy·형식 검사·release 빌드 통과. 네이티브 렌더러에서 로그인 9장, 소셜·프로필·사진 16장 확인.
- [PR #16 CI](https://github.com/dotgofgist/DotGwebsite/actions/runs/43626509382) 정적 검사·DB 검사·운영 빌드 통과. 새 네이티브 API E2E는 데스크톱과 모바일 모두 통과했습니다. 전체 E2E는 63개 통과, 2개 재시도 후 통과, 기존 색상 대비 20개 실패입니다.
- 운영 배포 워크플로의 읽기 전용 스모크와 실제 기본 도메인 `https://www.dotgsince2023.co.kr`의 API 검사를 통과했습니다.
- 사용자가 승인한 임시 일반 계정 1개로 운영 도메인에서 로그인·쿠키 미사용·프로필 조회·잘못된 사진 거부·사진 업로드·DB 재조회·공개 WebP 조회·세션 갱신·사진 삭제·로그아웃을 모두 검증했습니다. 로그아웃 후 기기·사진 API·갱신 토큰의 401 거부도 확인했습니다. 해당 계정·프로필·기기 세션·Storage 객체가 모두 0개 남았음을 확인했습니다. 기존 사용자 데이터는 변경하지 않았습니다. 결과: `artifacts/ingame-native-login/production-check.log`, 스크립트: `C:/Users/user/projects/tools/dotg-native-production-check.py`.
- 단일 EXE의 압축 해제·전체 파일 해시 검사와 패키지에서의 로그인 화면 렌더러 검사를 통과했습니다. 보고서: `artifacts/ingame-native-login/package-report.json`.

배포용 실행 파일: `release/TSB-Multiplayer-NativeLogin.exe` (108,469,121 bytes). SHA-256: `b9eabedf4d3cf7016a66e54c019d29daab91c98b8de46331427e04c4f80c3601`. 기존 실행 파일은 그대로 보관합니다.
