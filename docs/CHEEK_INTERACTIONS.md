# 캐릭터 선택창 볼 당기기

## 눈 감기 표면·삼각존 보정 (2026-11-16)

15명의 `eyeBlink_L/R`에 감긴 눈의 매끄러운 깊이와 법선을 함께 적용한다. 기존에는 텍스처를 바꾸면서 열린 눈의 돌출된 메시와 법선을 유지해 코미 등의 눈 윤곽이 양각으로 남았다. 기본 메시·UV·본·스킨 가중치와 다른 표정은 유지하고, 깜박임 강도에 따라 눈 영역의 굴곡을 줄인다. 겹친 표면은 원래 깊이 차이의 0.5%를 남겨 완전히 같은 평면에서 깜박이는 현상을 방지한다.

눈꼬리의 삼각존은 위쪽 속눈썹 선뿐 아니라 아래쪽 날개까지 덮고, 반쯤 감긴 단계부터 원래 위치의 무늬를 제거한다. 볼의 홍조·하트는 눈 바깥쪽에서 보호하며, 힐데의 안경과 우이의 머리핀은 기존 볼 보호 마스크를 공유해 메시·법선·텍스처 변경에서 제외한다. 아야의 머리카락에 가려진 눈은 기존 처리를 유지한다.

`*.morphs.json`의 선택적 `normal_indices`/`normal_targets`는 절대 법선과 가장자리 가중치를 저장한다. 런타임은 UV 경계에서 복제된 각 정점의 실제 기본 법선으로부터 델타를 계산하므로 열린 눈의 음영이 감긴 눈에 남지 않는다. 이 데이터가 없는 기존 모델도 계속 로드된다.

재현 순서: Blender에서 `fix_blink_surfaces.py` → `export_meshy_morphs.py -- artifacts/blink-surface-fix/candidate` → `verify_blink_surfaces.py`를 실행한다. 후보의 눈 상태를 `review_blink_surfaces.py`로 확인한 뒤 `publish_blink_surfaces.py`와 `export_meshy_morphs.py -- assets/characters`로 반영한다. 게시 스크립트는 원본이 작업 중 변경되었으면 중단한다. 백업·확대 렌더·GPU 캡처·검증 로그는 `artifacts/blink-surface-fix/`에 보관한다.

배포본은 `release/TSB-Multiplayer-BlinkFix.exe`다. 15명의 기본 메시·UV·본·스킨 가중치·다른 표정 보존 검사, 법선/UV 중복 정점 테스트 2개, Clippy·포맷 검사와 실제 배포본의 눈 상태 GPU 화면 45장을 통과했다. 단일 실행 파일에서 추출한 1,333개 파일의 해시를 확인했으며 상세 결과는 `artifacts/blink-surface-fix/final-verification.json`에 있다. 작업 중 갱신된 클로에의 팔·본도 최신 원본 기준으로 보존했다.

## 현재 적용: 볼 피부 분리 (2026-11-15)

15명의 볼 피부를 독립된 좌우 가중치로 제어한다. 머리·상체를 끌고 가던 회전과 입 반응 변형을 제거하고, 눈·입·머리카락·장식은 고정한다. 힐데의 안경과 우이의 머리핀도 별도로 보호한다. 클릭 반경은 0.10에서 0.14m로 넓혔으며 드래그 감도와 가로 ±0.12/세로 ±0.07의 상한은 유지한다. 피부를 조금 앞쪽으로 들어 올리고, 넓은 중심부에서 가장자리까지 완만하게 감쇠시켜 볼 끝을 둥글게 만든다. 좌우 중심은 각 모델의 실제 볼 표면에서 따로 지정한다.

얼굴의 큰 삼각형은 원래 표면·UV·법선·23개 표정을 보존하면서 세분화했다. `*-cheeks.json`에는 FBX 원본 좌표별 가중치와 법선 보정용 기울기를 저장하고, Blender 파일에는 `cheek_weight_L/R` 속성과 동일한 볼 표정 컨트롤을 넣었다. 얼굴을 실제로 잘라 틈을 만들지 않으며, 선택 미리보기의 복제된 메시만 실행 중에 변형한다. 기존 손·소매와 함께 수정된 나이아의 땋은 머리, 비비의 추가 공격, 이드의 탈것은 유지한다.

눈 감기는 15명 각각 원본 윗속눈썹의 위치·두께·곡선을 샘플링하여 아래로 내린다. 열린 눈/반쯤 감긴 눈/완전히 감긴 눈의 세 단계를 사용하며, `browDown`을 깜박임 강도의 75%로 연결한다. 클로에의 하트·홍조·앞머리와 아야의 눈을 가리는 머리카락을 보존한다. `--selection-review --blink-review --hud-review-small`로 45개 상태를 확인할 수 있다.

리뉴아는 기존 손 정리 과정에서 함께 잘린 외투를 원본에서 복구했다. 손가락 영역을 제외한 12,728개 면을 별도의 스킨 메시로 복원했으며, 새 손 리그와 1인칭 소매는 유지한다. 월드 모델의 임시 소매는 원래 외투가 대신 표시된다.

재현 스크립트는 `refine_face_triangles.py`, `refine_cheek_meshes.py`, `build_cheek_masks.py`, `repaint_natural_blinks.py`, `restore_renua_coat.py`다. 모델의 다른 편집과 병행할 때는 현재 리그·동작·장식을 유지한 채 얼굴 데이터만 합쳐야 한다. 검증과 원본 백업은 `artifacts/cheek-isolation/`에 보관한다.

배포본은 `release/TSB-Multiplayer-FacialRig.exe`다. 자동 테스트 259개(기존 무시 항목 2개), 전체 Clippy·포맷, 15명 Blender/FBX 왕복 검사를 통과했다. GPU 검사는 볼 105장·깜박임 45장·손 16장과 실제 단일 EXE의 클로에/우이/힐데 21장, 총 187장이다. 패키지에서 실제로 추출한 1,327개 파일의 해시도 일치했다. 상세 결과는 `artifacts/cheek-isolation/final-verification.json`, 실행 파일 SHA-256은 `7b3b8b52d6d8cbb4ca02c095ee458c667e8936c4584892ad4d84afd90ef87ea8`이다. 아래 항목들은 이전 구현의 이력이다.

2026-11-14. 현재 로스터 15명의 기본 의상 원작 `Touch_Idle`과 `Touch_End`를 조사하고 3D 미리보기에 대응시켰다.

## 사용

큰 3D 미리보기의 왼쪽 또는 오른쪽 볼을 마우스 왼쪽 버튼으로 누른 채 드래그한다. 이동 방향과 거리에 따라 그쪽 볼이 늘어나며 고유 반응이 반복된다. 버튼을 놓으면 볼이 탄성 복원되고, 해당 캐릭터의 놓기 반응이 한 번 재생된 뒤 대기로 돌아간다. 반응 중 다시 잡을 수도 있다. 단순 클릭에는 놓기 반응을 재생하지 않는다.

2026-11-15 자동 복귀 조정: 놓기 반응은 최대 3초 재생한 뒤 0.35초 동안 부드럽게 기본 자세로 돌아간다. 3초보다 짧은 반응은 원래 길이를 유지한다. 볼을 계속 잡고 있는 동안에는 당김 반응이 유지된다.

2026-11-15 보정: 같은 드래그 거리의 볼 변위와 최대 길이를 기존의 절반으로 줄였다(모델 좌표 가로 ±0.24, 세로 ±0.14). 눈을 납작하게 만드는 표정 변형을 제거하고 눈 테두리까지 고정해, 당기거나 놓을 때 눈 모양이 늘어나지 않게 했다. 머리·팔·입과 눈물 반응은 유지한다.

같은 날 추가 조정으로 최대 길이를 다시 절반으로 줄였다. 현재 상한은 모델 좌표 가로 ±0.12, 세로 ±0.07로 최초 구현의 1/4이다. 상한에 도달하기 전 드래그 감도는 유지한다.

초상화·방향키로 캐릭터 변경, 같은 초상화 다시 선택, 채팅·설정 메뉴 열기, 선택창 닫기, 창 포커스 상실·창 밖 이동·크기 변경은 드래그를 취소한다. 클릭 판정은 실제 움직이는 머리의 볼 위치를 카메라로 투영한다. 얼굴 비율이 다른 나이아·에르핀·리뉴아·버터는 별도 좌표를 사용한다.

선택 미리보기의 독립 메시만 변형한다. 공유 FBX, 경기 중 캐릭터, 1인칭 팔, 서버 상태와 전투 입력에는 영향을 주지 않는다.

## 조사 결과

원작 제작자는 EPID Games, 공개 참고자료 호스트는 Cheeky 커뮤니티 뷰어다. 아래 링크의 **standing / Normal** 스킨을 사용했다. 각 파일의 Spine 버전은 4.1.08이며 모든 대상에 `Touch_Idle`과 `Touch_End`가 실제 존재한다. 초 단위 시간은 원본 타임라인에서 읽은 값이다.

| 캐릭터·원작 자료 | 당기는 동안 반복 | 놓기 반응 | 원작에서 관찰한 구분 요소 |
|---|---:|---:|---|
| [비비](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-vivi-vivi) | 0.48443 | 4.42907 | 눈을 감고 버티기 → 눈을 뜨며 볼 옆 손·입 반응 |
| [나이아](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-naia-naia) | 0.26667 | 3.2 | 눈물이 맺힌 놀란 눈 → 눈을 감고 얼굴 쪽 손 |
| [에르핀](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-erpin-erpin) | 0.53333 | 2.4 | 눈을 찡그린 자세 → 고개·상체·팔을 움직이는 항의 |
| [이드](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-ed-ed) | 2.13333 | 4.0 | 앉은 상태의 느린 반응·눈물, 동반 드론 표정 |
| [아야](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-aya-aya) | 0.4 | 4.0 | 놀란 눈 → 눈을 가늘게 뜨고 입·손 반응 |
| [피코라](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-picora-picora) | 0.8 | 4.0 | 눈을 꼭 감기 → 눈을 뜨고 볼 쪽으로 손을 올림 |
| [클로에](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-chloe-chloe) | 0.53333 | 1.6 | 눈 찡그림 → 눈물·얼굴 옆 손, 세바스티안도 반응 |
| [리뉴아](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-renewaawaken-renewaawaken) | 2.13333 | 4.8 | 놀란 눈과 모은 손 → 눈을 가늘게 뜨고 한 손을 얼굴 쪽으로 |
| [마요](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-mayo-mayo) | 0.4 | 4.0 | 반쯤 감은 눈·모은 손, 놓은 뒤 완만한 고개 반응 |
| [벨라](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-vela-vela) | 1.6 | 6.4 | 눈을 감고 손을 모음 → 긴 부유 자세·고개 반응 |
| [우이](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-ui-ui) | 0.4 | 2.8 | 눈 찡그림·올린 팔 → 상체 기울임·팔 내림, 개구리도 반응 |
| [버터](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-butter-butter) | 0.26667 | 3.2 | 양눈을 꼭 감기 → 볼에 손·눈물·고개 기울임 |
| [코미](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-kommy-kommy) | 0.4 | 4.26667 | 감긴 눈 → 눈물을 머금은 열린 눈·작은 입 |
| [힐데](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-hilde-hilde) | 0.53333 | 2.4 | 눈을 꼭 감기 → 놀란 눈·입가로 올린 손 |
| [시온](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-xxionx-xxionx) | 0.66667 | 3.33333 | 한쪽 눈 찡그림 → 양눈 눈물·입·고개 반응 |

정확한 URL·원본 SHA-256은 `artifacts/cheek-interaction/research/sources.json`, 전체 애니메이션 시간·본 이름·표정 attachment 타임라인은 `motion-report.json`에 보관한다. 원작 15명 각각 기본/당김 2시점/놓기 2시점을 직접 렌더링한 비교 이미지는 `artifacts/cheek-interaction/original-motions.png`다.

## 반영 방식과 범위

- `scripts/extract_cheek_motions.cjs`가 원본 머리·상체·팔 회전과 손 위치를 30Hz로 샘플링한다. 원본 데이터의 반복/종료 시간을 보존하되 선택창의 놓기 반응은 최대 3초로 제한하고, 3D 팔 길이를 유지하는 두 관절 IK로 손 위치를 맞춘다. 좌우 볼 반응은 좌우를 반전한다.
- 원작 표정 그림의 교체는 현재 모델의 입 변형과 눈물 메시로 근사한다. 눈을 세로로 압축하던 근사는 눈이 길쭉해 보이는 문제 때문에 제거했다. 머리카락·모자·안경과 눈 전체는 볼 변형에서 제외하고 원래 모양을 유지한다.
- 2D 리그와 현재 3D 리그의 구조가 다르므로 원본 픽셀·실루엣을 그대로 복제한 결과는 아니다. 손가락 모양, 원작 소품 전체, 드론/세바스티안/개구리의 별도 얼굴 애니메이션과 원작 음성은 포함하지 않는다. 기존 3D 소품은 해당 본을 따라 움직인다.
- Spine 실행 코드·텍스처는 조사 폴더에만 있다. 게임은 `assets/characters/cheeks/motions.json`의 수치 데이터를 실행 파일에 포함하므로 추가 다운로드가 필요 없다. 원본 캐릭터 FBX·Blend는 이 기능에서 수정하지 않는다.

## 검증

`cargo test --offline --no-default-features cheeks`는 15명 클립·표정·손 데이터, 드래그 제한, 놓을 때 연속성·자동 종료, 단순 클릭·취소·재잡기와 배율을 검사한다.

`tsb --cheek-review`는 실제 클릭 판정·드래그·놓기 함수를 통해 15명 각각 기본, 왼쪽 당김 2시점, 놓기 2시점, 오른쪽 당김, 원상 복구의 **105개 GPU 화면**을 저장한다. 원상 복구는 강제 취소 없이 실제 프리뷰 업데이트의 타이머가 끝날 때까지 기다려 검사한다. 양쪽 최대 대각선 당김과 일반 드래그를 포함하고, 매 화면에서 눈 정점의 위치·법선 보존과 변위 상한을 검사한다. `--hud-review-small`을 붙이면 960×640에서 검사한다. 상태·변형 정점 수와 화면상 볼 위치를 로그에 남긴다. `scripts/cheek_contact_sheets.ps1`은 15명 비교표 3장을 만든다.

최종 검증 빌드에서 960×640과 1920×1200 각각 105개, 총 210개 GPU 화면 검사를 통과했다. 전용 테스트 4개, 전체 타깃 Clippy 경고 금지 검사와 기능 모듈 포맷 검사도 통과했다. 두 GPU 실행 모두 종료 코드 0이며 엔진 오류·패닉이 없다. 원작 데이터 추출을 다시 실행해 결과 해시가 같음을 확인했다.

검사 로그·화면·빌드 및 데이터 해시는 `artifacts/cheek-interaction/validation.json`과 같은 폴더에 보관한다. 검증한 별도 빌드는 `artifacts/cheek-interaction/Play.cmd`로 프로젝트 루트에서 실행된다. 공유 기본 실행 파일의 잠금을 피하기 위해 별도 출력했으며, 단일 EXE 배포본은 이 작업에서 재패키징하지 않았다.

### 2026-11-15 길이·눈 보정 검증

전용 테스트 4개, 클라이언트 Clippy 경고 금지 검사, 변경한 Rust 파일 포맷 검사를 통과했다. 960×640에서 15명 × 7상태, 총 105개 GPU 화면의 일반·최대 당김, 복원과 눈 정점 위치·법선 보존을 검사했다. 종료 코드 0, 엔진 오류·패닉 0이다. 소스 백업·변경 diff·로그·캡처·SHA-256은 `artifacts/cheek-stretch-fix/`에 보관한다. 실행 중인 기본 `tsb.exe` 잠금을 피한 최신 수정본은 `artifacts/cheek-stretch-fix/Play.cmd`로 실행한다. 단일 EXE 배포본은 재패키징하지 않았다.

### 2026-11-15 자동 복귀 시간 검증

놓기 반응 최대 3초와 기본 자세 복귀 0.35초를 적용했다. 15명 전체의 시간 제한·부드러운 전환·느린 프레임·다시 잡기를 포함한 전용 테스트 5개와 클라이언트 Clippy 경고 금지 검사를 통과했다. 960×640의 105개 GPU 화면 검사도 종료 코드 0으로 완료했다. 복원 화면은 기본 자세가 실제로 렌더링된 뒤 저장하며, 비비·벨라의 캐릭터 영역은 기본 화면과 픽셀 차이 0을 확인했다. 같은 파일에 병행 반영된 최대 당김 길이 1/4 조정도 유지한다. 검증 기록과 실행 파일은 `artifacts/cheek-idle-return/`, 실행은 그 안의 `Play.cmd`를 사용한다. 단일 EXE 배포본은 재패키징하지 않았다.

### 최대 길이 추가 축소 검증

현재 상한 ±0.12/±0.07에서 관련 테스트 5개와 포맷·빌드 검사를 통과했다. 960×640의 15명 × 7상태, 105개 GPU 화면에서 양쪽 최대 당김·눈 보존·놓기·자동 복귀를 검사했으며 종료 코드 0이다. 최신 수정본은 `artifacts/cheek-stretch-limit/Play.cmd`로 실행한다. 검증 결과·캡처·실행 파일 SHA-256은 같은 폴더의 `validation.json`과 함께 보관한다.
