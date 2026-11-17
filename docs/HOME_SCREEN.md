# 홈 화면 / 2026-11-12

## 구성과 동작

일반 실행 시 홈 앞에 EPID Games·DOTG의 흰 배경 크레딧과 로그인/회원가입/게스트 선택 화면이 열린다.
방 목록의 웹 계정 연결 버튼은 제거했다. 게스트의 홈 프로필·소셜도 인게임 계정 화면을 열며, 그 안의 로그인 버튼에서 로그인 선택 화면으로 이동한다.
시작 흐름, 웹 인증 연계, 로고 출처와 검증은 [시작 화면 문서](STARTUP.md)에 기록했다.

참고 화면의 왼쪽 메뉴·왼쪽 아래 채팅·오른쪽 위 계정 배치를 Fyrox UI로 구현했다. 일러스트는 화면을 채우되 종횡비를 보존하며 오른쪽 캐릭터 쪽을 기준으로 잘라낸다. 메뉴와 클릭 영역은 같은 배율을 사용한다.

- 플레이 → 방 선택 → 방 만들기/참여하기 → 대전 로비 → 게임 시작 → 영웅 선택 확정 → 경기. 로비의 별도 영웅 선택은 확정 후 로비로 돌아온다. 선택창의 Esc는 로비, 로비의 Esc는 방 목록, 방 목록의 Esc는 홈으로 복귀한다. [방 선택 상세](ROOM_BROWSER.md), [로비 상세](LOBBY.md).
- 게임 종료 → 엔진 이벤트 루프 종료. F10도 유지한다.
- 전투훈련은 연습 방을 열고, 소셜·프로필은 게임 안의 친구 관리·프로필 편집 화면을 연다. [기능과 서버 적용](INGAME_ACCOUNT.md). 영웅갤러리, 전리품 상자, 하이라이트는 표시 전용이다.
- 친구 채팅은 표시 전용이다. 우측 상단 플레이어 카드는 연결된 DotG 계정의 닉네임·@아이디·프로필 사진을 표시한다. 사진이 없거나 다운로드에 실패하면 선택한 프로필 색상과 닉네임 첫 글자를 표시한다.
- 홈의 소셜·프로필 버튼은 인게임 화면을 열고, 계정이 연결되어 있으면 바로 조회·편집한다. 최초 로그인과 만료 후 재연결만 기존 브라우저 인증을 사용한다. 웹에서 게임으로 돌아오면 프로필을 즉시 다시 확인하며, 연결 중에는 3초, 연결 후에는 30초 간격으로 갱신한다. 방을 나가도 계정 표시는 유지하고, 기기 연결이 만료되면 지운다. 웹 브라우저 로그인과 게임 계정 연결은 별개이며 기기 토큰은 메모리에만 보관한다.
- 홈에서 게임 입력·IME·채팅·커뮤니케이션 휠을 처리하지 않는다. Tab/위·아래 방향키는 실제 동작 두 개만 순환하며 Enter로 실행한다.
- `src/presentation/home.rs`에 화면과 클릭 영역을 분리했다. 홈은 마지막에 생성한 전체 화면 UI이며 기존 경기 HUD 위에 표시한다.
- 창 테두리를 드래그하는 동안에도 `WindowEvent::Resized`에서 홈 배경의 종횡비·메뉴 배치·클릭 영역을 즉시 갱신하고 다시 그린다. Windows의 크기 조절 루프가 일반 게임 업데이트를 멈추더라도 UI 메시지와 레이아웃을 먼저 반영한다. 최소화로 전달되는 0 크기는 무시한다.

## 게임용 로고와 원본 출처

- 현재 적용: `assets/ui/home/trickcal-shooting-balddagu-logo.png` (1672×941, 투명 PNG). 내장 image_gen으로 원작 로고를 참고해 생성했다. 큰 한글 ‘트릭컬’ 아래에 작고 단색 갈색인 영문 ‘SHOOTING BALDDAGU’를 넣었다.
- 처음 생성한 큰 한글 ‘슈팅 볼따구’ 부제는 사용자의 후속 요청에 따라 작은 갈색 영문으로 변경했다. 별도 흰색 영문 UI 텍스트는 제거해 부제가 로고 안에서만 보이게 했다. 로고는 종횡비를 유지하며 기준 너비 280으로 표시한다.
- 최종 프롬프트 전문: `assets/ui/home/logo-english-prompt.txt`. 기존 원작 로고 파일은 참고 기록으로 보존한다.

- 공식 사이트: https://www.trickcal.com/
- 공식 SVG: https://www.trickcal.com/assets/logo-BHL9MqGO.svg (2026-11-12 확인)
- 원본 보관: `assets/ui/home/trickcal-official.svg`
- 최초 적용 원본: `assets/ui/home/trickcal-official.png`. 공식 SVG 안에 포함된 599×337 PNG 바이트를 그대로 추출한 참고 자료이며 현재 런타임은 위 게임용 생성 로고를 사용한다.
- 트릭컬 로고와 원작 캐릭터의 권리는 EPID Games에 있다. 홈 아래에는 팬게임 표시를 둔다.

## 일러스트와 수정 기록

내장 image_gen으로 기존 원작 전신 참고 이미지 `assets/references/{erpin,butter,komi}-fullbody.png`를 참고해 새 팬게임 일러스트를 제작했다. 크리스마스 요소 없이 에르핀·버터·코미가 엘리아스 정원에서 피크닉하는 장면이다.

- 최초안: `assets/ui/home/elias-picnic.png`, 프롬프트 `assets/ui/home/generation-prompt.txt`.
- 사용자 피드백: 배경이 화려하고 대비가 짙음 → 배경 채색 단순화. 후속 피드백: 단순화를 너무 많이 함.
- 적용본: `assets/ui/home/elias-picnic-soft.png` (현재 1671×941). 캐릭터와 구도는 유지하고 정원·세계수·돌길의 입체감과 디테일을 살리면서 강한 빛줄기·반짝임·미세한 하이라이트·짙은 명암을 줄였다.
- 배경 채색 수정 프롬프트 전문: `assets/ui/home/revision-prompt.txt`. 내장 도구 모드를 사용했으며 API/CLI 생성은 사용하지 않았다.

### 2026-11-15 에르핀 소품 중복 수정

- 내장 `image_gen` 편집으로 머리 뒤의 중복 지팡이 장식을 제거하고 겹쳐 있던 왕관을 작은 왕관 하나로 정리했다. 오른쪽 나무에 기대 둔 지팡이 하나는 유지했다.
- 기존 적용본과 `assets/references/erpin-fullbody.png`를 참고했다. 세 캐릭터의 표정·자세, 피크닉 구도, 정원과 부드러운 색감은 유지했다.
- 수정본은 `assets/ui/home/elias-picnic-soft.png`에 적용했다. 홈·방 선택·로비가 같은 경로를 사용하며 패키징에도 자동 포함된다.
- 최종 수정 프롬프트 전문: `assets/ui/home/erpin-props-correction-prompt.txt`. 수정 전 이미지: `artifacts/home-illustration-correction-20261115/elias-picnic-soft-before.png`.
- `target/debug/tsb.exe --solo --home-review`가 종료 코드 0으로 완료됐다. 실제 1920×1200 홈 GPU 캡처에서 왕관·지팡이가 각각 하나이고 메뉴와 기존 구도가 유지되는 것을 확인했다. 캡처와 실행 로그: `artifacts/home-illustration-correction-20261115/`.
- 배포용 빌드: `release/TSB-Multiplayer-HomeArtFix.exe`. `cargo build --locked --offline --release --bin tsb --bin tsb-eos-check --bin tsb-eos-peer-check` 후 기존 단일 EXE 패키징 스크립트로 제작했다.
- 새 EXE의 `--verify-only`가 종료 코드 0으로 통과했고, 추출된 배경의 SHA-256이 프로젝트 수정본과 일치했다. 패키지 정보: `artifacts/home-illustration-correction-20261115/package-report.json`.

## 검증

```powershell
cargo test --locked --bin tsb presentation::home
cargo run --locked --bin tsb -- --home-review
cargo run --locked --bin tsb -- --home-review --home-review-small
cargo run --locked --bin tsb -- --home-review --home-review-wide
```

홈 검증은 에셋 로드와 홈 → 선택 → 홈 → 선택 확정 → 경기 흐름을 검사하고 단계별 실제 GPU PNG를 `artifacts/preview-home-*`에 저장한다. UI가 화면 전체를 덮는 홈 캡처에서는 3D 화면이 알파 합성으로 다시 비치지 않도록 UI 결과를 그대로 사용한다.

1920×1200, 960×640, 1920×800에서 글자·메뉴·채팅·배너 배치를 직접 확인했다. 실제 창에서 미구현 메뉴 클릭 시 유지, 플레이 클릭 시 선택창, Esc 시 홈 복귀, 게임 종료 클릭 시 정상 종료를 확인했다. fmt·Clippy(경고 금지)·클라이언트/서버 빌드·홈 클릭 영역 테스트를 통과했다. 검사 로그는 `artifacts/home-screen/`에 있다. 기존 테스트 87개는 샌드박스에서 로컬 UDP 연결 차단으로 일부 실패했지만 로컬 통신이 가능한 실행에서는 전체 통과했다.

## 2026-11-15 실시간 창 크기 변경

Windows의 크기 조절 모드를 유지한 채 960×640 → 976×640 → 992×640으로 변경했을 때, 조절 종료 전에 배경·메뉴·프로필·하단 패널이 새 크기로 그려지는 것을 확인했다. Esc로 원래 크기를 복원하는 동작도 확인했다. 레이아웃만 갱신하면 이전 크기의 그리기 데이터가 남으므로, UI 메시지 처리 뒤 시간 간격 0의 `ui.update`로 그리기 데이터까지 다시 만든다.

`live_resize_rebuilds_draw_data_without_a_game_update`는 게임 업데이트 없이 연속 크기 변경, 실제 그리기 데이터, 메뉴 클릭 영역, 0 크기 및 홈 밖에서의 무시 동작을 검사한다. 클라이언트 테스트 34개 통과(오디오 장치 테스트 1개 제외), fmt·Clippy(경고 금지)·릴리스 빌드·단일 EXE `--verify-only`를 통과했다. 결과 파일은 `release/TSB-Multiplayer-LiveResize.exe`, 실행 로그는 `artifacts/live-home-resize/verified2-stdout.log`에 있다.

## 2026-11-15 프로필 동기화

홈 배너가 `Session::Solo`의 `PLAYER 1`과 고정 초상화만 사용하던 문제를 수정했다. `room_directory`가 이름·핸들·프로필 색상·사진 URL을 받아 화면에 전달한다. 사진은 작업 스레드에서 크기 제한을 적용해 다운로드·디코딩하고, 게임 스레드에서는 준비된 픽셀을 텍스처로 연결한다. 사진 요청에 기기 토큰을 보내지 않는다.

사진을 보내려면 DotG 웹 프로젝트의 `20261115090000_add_native_player_profile.sql` 마이그레이션과 `/api/game/rooms` 수정본도 배포해야 한다. 이전 서버와 연결하면 닉네임은 표시되며, 사진 정보가 없는 동안은 첫 글자 아이콘을 사용한다.
