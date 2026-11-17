# 라운드 종료 연출

## 원작 조사

- [인벤 트릭컬 리바이브 리뷰 (2023-10-06)](https://www.inven.co.kr/webzine/news/?news=289078)의 [승리 스크린샷](https://static.inven.co.kr/column/2023/10/06/news/i8260456839.jpg): 연두→초록색 둥근 VICTORY, 짙은 녹색 테두리, 금색 별 세 개, 원형 엠블럼과 잎/리본 장식. 큰 캐릭터와 보상 목록이 함께 나온다.
- [루리웹 원작 플레이 캡처 (2023-10-16)](https://bbs.ruliweb.com/community/board/300143/read/63399245)의 [패배 스크린샷](https://i1.ruliweb.com/img/23/10/16/18b372f625025106.jpg): 금이 간 짙은 보라색 판, 밝은 회보라 DEFEAT, 위쪽 작은 대회 문장. 캐릭터와 반성문이 함께 나온다.
- 두 자료의 결과 장식을 직접 확인했다. 게임 모드와 촬영 시점에 따른 변형이 있을 수 있다. 슬로모션의 정확한 원작 길이는 정지 화면으로 확인할 수 없으므로 아래 시간과 팝업 움직임은 TSB용 각색이다.

## 구현

- 종료 직후 실제 시간 0.9초 동안 마지막 이동, 점프, 투사체와 캐릭터 모션을 0.16배속으로 이어간다. 결과가 정해진 뒤 새 공격, 피해, 점령, 부활은 진행하지 않는다. 지형 충돌은 유지한다.
- 0.9초 후 결과 엠블럼과 `라운드 승리`/`라운드 패배`가 0.42초 동안 커졌다가 안착한다. 배경은 서서히 어두워지고 마지막 0.35초에는 배너가 사라진다.
- 첫 라운드 및 연장 라운드는 B 점령 시 공격팀 승리, 시간 만료 시 수비팀 승리다. 최종 라운드는 기존 경기 판정에 따라 `최종 승리`/`최종 패배`/`무승부`를 표시한다.
- 기존 실제 시간 6초의 공수 교대, 8초 준비, 5초 최종 결과 후 같은 로비로 복귀하는 흐름을 유지한다.
- 서버의 `phase_time`과 기존 스냅샷의 `result`로 표시 시점을 공유한다. 패킷 크기와 전송 주기를 바꾸지 않는다. `result`는 Intermission에서 직전 라운드 승자, Finished에서 최종 승자이며 새 라운드 시작 때 지운다. 늦게 참여해도 현재 시점부터 표시한다.
- 점수판(Tab)과 채팅(Enter)을 유지한다. 결과 화면은 입력을 가로채지 않고, 다음 준비 단계에서 자동으로 제거된다. 영웅 선택창과 데스캠보다 결과를 우선한다.
- `assets/ui/round-results/victory.png`, `defeat.png`: 내장 image_gen으로 각각 생성한 1536×1024 RGBA 이미지. 투명도를 그대로 보존해 복사했다. 원본 스크린샷은 비교 자료이며 게임에 포함하지 않는다. 한글 문구는 게임의 ONE Mobile POP 글꼴로 표시한다.
- 패키징 스크립트도 두 런타임 이미지를 포함한다.

## 검증 및 재현

배포 실행 파일: `release/TSB-Multiplayer-RoundResults.exe`. 기존 배포본을 보존하고 새 이름으로 생성했다. 게임, 생성 그래픽, 기존 런타임과 게임용 EOS 설정을 포함한다.

```powershell
cargo test --locked --offline --no-default-features
cargo clippy --locked --offline --all-targets -- -D warnings
cargo build --locked --offline --bins
target/debug/tsb.exe --solo --auto-start --round-results-review
target/debug/tsb.exe --solo --auto-start --round-results-review --hud-review-small
```

라운드 전환 테스트는 감속 거리와 실제 시간, 피해/점수 동결, 투사체 지형 충돌, 공격/수비 결과, 연장전 초기화, 무승부 로비 복귀 및 실제 localhost UDP/늦은 참가를 검증한다. GPU 검증은 감속 구간, 라운드 승패, 최종 승패, 무승부, 다음 준비 단계의 7장을 저장한다. 로그와 조사 자료는 `artifacts/round-results/`, GPU PNG는 `artifacts/preview-round-result-*.png`에 저장한다.

2026-11-14 검증: 헤드리스 184개, 기본 기능 포함 217개 테스트 통과. 기본 마이크·스피커를 요구하는 기존 보이스 장치 테스트 1개는 기본 ignore 상태를 유지했다. 최종 UI 수정 후 데스크톱 테스트 27개를 재실행해 통과했다. rustfmt, 전체 대상 Clippy 경고 0, debug/release 빌드를 통과했다. 1920×1200과 960×640에서 각 7개 화면의 GPU 검증이 완료됐다. 작은 창에서는 LayoutTransform의 자식 클리핑을 확인해 RenderTransform으로 수정했으며, 결과 중 닫힌 채팅·보이스 패널은 숨기고 음성 연결 및 Enter 채팅은 유지한다.

## 이미지 생성 프롬프트

방식: 내장 `image_gen`, `transparent_background=true`. 각 원작 스크린샷을 스타일 참고 이미지로 지정하고 한 장씩 별도 생성했다.

### Victory

```text
Use case: stylized-concept. Asset type: transparent raster victory HUD badge for a Trickcal fan game. Reference image is a style/composition reference ONLY: use the green VICTORY crest visible in the upper-right of the reference, not its whole screen. Generate a single polished wide emblem on actual transparent background. Match Trickcal's thick rounded friendly cartoon lettering: huge exact word "VICTORY" with lime-yellow upper faces blending into vivid mint-green lower faces, thick very dark green outline, subtle chunky depth and small glossy highlights. Three golden puffy five-point stars arch above the lettering, middle star bigger. Behind the letters an airy circular pale-lime crest containing a very faint stylized fairy castle/tree silhouette. Under the word a light lime curled leafy ribbon flourish and central four-point sparkle. Symmetrical composition. Clean readable silhouette, hand-drawn mobile-game UI quality. Artwork centered with generous transparent space on all edges, wide 3:2 canvas, emblem about 90% width and 70% height, full uncropped decorative ends. ONLY text is VICTORY. No characters, no screenshot scene, no stage labels, no buttons, no rewards, no watermark, no checkerboard, no opaque background. This is the winning counterpart of a dark purple stone DEFEAT badge.
```

### Defeat

```text
Use case: stylized-concept. Asset type: transparent raster defeat HUD badge for a Trickcal fan game. Reference image is a style/composition reference ONLY: use the dark purple DEFEAT plaque visible on the right of the reference, not its whole screen. Generate a single polished wide emblem on actual transparent background. Match Trickcal's chunky rounded cartoon UI. Huge exact word "DEFEAT" with cool pale lavender-gray faces, thick dark purple outlines, subtly rough edges and light inset shadow, centered on a horizontal elongated hexagonal slate plaque with clipped pointed ends, double beveled purple-black border and a few small cracks. At top center a small purple medallion containing a cute cracked trophy and two tiny rounded cloud shapes, unlettered. A little subdued gray-lilac leafy curl below makes the emblem consistent with a green leafy VICTORY counterpart. Mood rueful and cute. Muted desaturated plum, smoky charcoal, lavender. Clean readable silhouette and crisp mobile-game asset quality. Artwork centered with generous transparent space on all edges, wide 3:2 canvas, emblem about 90% width and 60% height, full uncropped decorative ends. ONLY text is DEFEAT. No characters, no parchment apology, no additional lettering, no scene, no button, no watermark, no checkerboard, no opaque background.
```
