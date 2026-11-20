# 캐릭터 아트·모션 / 2026-11-13

## 아야 양산 접힘·검·언월도 / 2026-11-15

아야의 별도 Meshy 양산에 접힘·꽃잎 모으기 모프와 입체 칼날 3종을 추가했다. `Attack1_1` 두 번 뒤 `Attack2_1`을 재생하며 전신·1인칭·데스캠에서 원작의 꽃잎 및 칼날 전환 시점을 공유한다. 기존 피해·공격 간격은 유지한다. 아래 과거 기록의 내장 양산 및 무기 변신 미지원 설명은 현재 아야에 적용되지 않는다. [모델·동작·재생성·검증](AYA_WEAPON_TRANSFORM.md).

현재 모델은 [Meshy 에셋 제작 절차](MESHY_ASSETS.md)와 이후 캐릭터별 수정 문서를 기준으로 한다. 아래 Tripo·20본 모델 설명은 교체 전 이력이며, 해당 구형 모델 전용 일회성 보정·검증 스크립트는 2026-11-20 정리에서 폐기했다.

당시 사용자 제공 Tripo 모델은 버터·나이아·에르핀·리뉴아 4명이었다. 버터는 70본, 새로 적용한 3명은 원본 65본과 게임 호환 보조 본 7개를 합친 72본 리그를 사용한다. 나머지 11명은 기존 20본 리그다. 모두 11종 동작이 포함된 FBX다. [로컬 아트 목록](../assets/characters/index.html)에서 캐릭터별 삼면도·렌더·Blender 원본·FBX·출처를 열 수 있다.

## 사도별 사망 모션 / 2026-11-14

나이아·에르핀·리뉴아의 기존 24fps 저장 오류를 30fps로 정정했다. 게임의 30fps 클립 경계와 실제 FBX 시간이 어긋나던 문제이며, 공통 내보내기와 에셋 로딩 검사에도 같은 기준을 적용한다.

15명의 원작 전투 `Die` 트랙을 각각 조사하고 몸·머리·사지 동작을 현재 3D 리그에 옮겼다. Death 구간 423–453은 정규화된 구간이며 실제 길이는 사도마다 1.833–3초다. 사망 탑뷰에서 모션을 끝까지 재생한 뒤 처치 전 녹화 데스캠을 시작한다. [출처·길이·재생성·검증](DEATH_REPLAY.md)을 참조한다. 기존 메시·가중치와 다른 동작 키는 보존했다.

## 원작 춤 적용 현황 / 2026-11-14

15명 모두 조사했으며, 현재 **5명만 원본 전용 춤을 확인하여 적용**했다. 비비·나이아·에르핀·이드·마요·벨라·우이·버터·코미·시온은 조사한 기본 스탠딩 원본에 `Dance` 트랙이 없어 기존 제작 춤을 유지한다. 이 결과가 원작의 다른 버전이나 별도 콘텐츠에도 춤이 없다는 뜻은 아니다. 미확인 모션이나 전투 `Victory`를 원작 춤으로 대신 표기하지 않는다.

| 사도 | 원본과 적용 클립 | 실제 반복 시간 |
|---|---|---|
| 아야 | [Normal / Dance_1](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-aya-aya) | 1.600초 |
| 피코라 | [Normal / Dance_1](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-picora-picora) | 1.067초 |
| 클로에 | [Normal / Dance_1](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-chloe-chloe) | 1.067초 |
| 리뉴아 | [Normal / Dance_1](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-renewaawaken-renewaawaken) | 2.667초 |
| 힐데 | [Normal / Dance_1](https://wiki.cheeky.tsuncloud.com/korea/spines/spines-spine-standing-hilde-hilde) | 1.600초 |

원본의 화면상 어깨·팔꿈치·손목·골반·무릎·발 궤적을 읽어 각 3D 리그에 옮겼다. 깊이 방향과 발 접지는 3D 체형에 맞춘 해석이며, 원작의 표정/소품 이미지나 텍스처를 복사하지 않는다. 아야는 모델에 붙어 있는 양산의 손잡이 자세를 유지한다. 285–405 프레임은 정규화된 저장 구간이며 `src/motion.rs`의 사도별 60Hz 주기가 실제 재생 속도를 결정한다. 실시간 및 데스캠 재생 모두 같은 모션 시계를 사용한다. G와 커뮤니케이션 휠의 기존 춤 조작을 그대로 사용한다.

재현: `node scripts/extract_dance_motions.cjs` → Blender `--background --python-exit-code 1 --python scripts/bake_dance_motions.py`. 원본 스켈레톤과 Spine 조사 런타임은 `artifacts/cheek-interaction/research`의 기존 자료를 사용한다. 춤 외 모든 애니메이션 키와 메시·가중치가 바뀌지 않았는지 베이크 과정에서 검사한다. 출처·전체 클립 목록은 `artifacts/dance-research/roster-audit.json`, 적용 전 백업은 같은 폴더의 `before`, 베이크 검증은 `asset-verification.json`에 있다. `--dance-review`는 실제 게임에서 15명 각각 네 시점의 춤을 캡처한다.

## 1인칭 무기 파지 / 2026-11-13

- **에르핀·우이 무기 크기 / 2026-11-20**: 확대된 1인칭 손에 비해 작던 지팡이를 캐릭터별로 조정했다. `weapons::first_person_scale()`에서 에르핀은 0.72→1.08(1.5배), 우이는 0.72→1.30(약 1.81배)를 사용한다. `first_person_position()`으로 오른쪽 아래 파지 위치와 대기 시 끝장식의 화면 여백을 맞췄다. 손바닥 접점·에르핀 보석 발사점·우이 클로버 회전은 확대된 무기 변환을 따르며, 월드 무기와 전투 수치는 유지한다. 검증은 `--weapon-review --erpin-only`, `--weapon-review --ui-komi-only`, `--ui-skill-review`로 재현한다.

- **전체 오른손 정정 / 2026-11-14**: 현재 15명 리그는 좌표 기준 이름을 사용해 `Hand.L`이 실제 오른손, `Hand.R`이 실제 왼손이다. `skeleton::arm_bone_names`로 1인칭 파지·인사와 월드 무기의 선택을 통일했다. Tripo 네 명은 원본 `mixamorig:RightHand` 손가락으로도 확인하며, 손바닥 접촉점과 손가락 굽힘 방향을 오른손에 맞췄다. 비비·클로에·코미·시온의 보조 손은 실제 왼손이다.
- 월드 무기는 애니메이션 갱신 후 손잡이 접촉점을 손 본에 맞춰 붙인다. 14명의 공격·스킬은 오른팔 중심으로 다시 구웠으며, 아야는 이미 실제 오른손에 연결된 내장 양산을 유지한다. 1인칭 팔 추출에서는 아야 원본 양산 재질을 제외해 별도 무기와 중복 표시하지 않는다. `scripts/correct_weapon_handedness.py`는 해당 회전 키만 수정하며 모델 형상·가중치·나머지 모션 키의 보존을 검사한다. 재생성용 `combat_rotations`도 같은 오른손 규칙을 사용한다.
- 전체 `--weapon-review`는 이제 기존 72개에 전원 외부 시점 대기·공격 30개를 추가한 102개 화면을 검사한다. 원본 손의 좌우, 손등과 손가락 굽힘, 손잡이 접촉 오차 1mm 미만을 확인한다. 변경 전 코드·에셋과 검증 로그·화면 모음은 `artifacts/right-hand-fix/`에 보관한다.

- 이전 에르핀 단독 정정은 원본 `mixamorig:RightHand` 손가락을 가진 실제 오른팔을 선택하고 넓은 소매·손바닥 방향을 보정했다. 이 선택 규칙을 위 전체 로스터 정정으로 통합했으며 에르핀 전용 소매·지팡이 각도 보정은 유지한다. `--weapon-review --erpin-only`와 `--greeting-review --erpin-only`도 유지한다. 당시 자료는 `artifacts/erpin-right-hand/`, 이전 방향 보정 이력은 `artifacts/erpin-hand-direction/`에 있다.
- 인사 소매 정정: 1인칭 팔은 원본 기준 자세에서 손·소매를 같이 들어 올리고 완만한 팔꿈치 굽힘과 전용 화면 배치를 사용한다. 월드의 구부러진 팔을 복사한 뒤 위팔만 세워 생기던 비틀림과 팔 분리 경계 노출을 줄였다. 원본 정점·UV·가중치·재질을 유지하며, `skeleton.rs`에서 FBX 기준 회전을 반영한다. `--greeting-review`는 15명×6단계와 Tripo 4명의 외부 시점, 총 94장을 검사한다.
- 아야의 양쪽 소매에 잘못 들어 있던 갈색 재질을 흰 기모노 재질로 정정했다. 당시 일회성 소매 보정은 각 팔 320면의 재질만 바꾸고 정점·가중치·손 피부·20본 리그를 보존한다. 활성 Blend·FBX·모델 미리보기·기존 3D 초상화와 생성 스크립트를 함께 갱신했다. 2D HUD 초상화는 그대로다.

- `first_person.rs`가 캐릭터 FBX의 팔·손에 가중치가 있는 삼각형만 별도 인스턴스로 표시한다. 원본 정점·UV·재질·스키닝과 에셋 파일을 유지하며, 15명 각각의 피부와 소매를 사용한다.
- `weapons::grips`의 무기별 접촉점에 손바닥을 맞춘다. Tripo 네 캐릭터는 손바닥 축을 손가락 본에서 계산해 원본 손가락을 구부리고, 나머지는 기존 둥근 손 메시를 사용한다. 비비·클로에·코미·시온은 왼손도 표시한다.
- 손과 무기는 반동·근접 공격·이동 흔들림·재장전 변환을 공유한다. **비비는 2026-11-15부터 어검술로 변경**했다. 손을 편 채 원작 손 제스처를 취하고, 등 뒤 수은검은 독립 무기 본의 궤적을 따른다. 2026-11-20부터 `Attack1_1`의 좌→우/우→좌 구간을 홀드 중 번갈아 재생하며, 탭은 좌→우 한 번만 수행한다. 수은검은 네 개이며 [전용 날개·E·Q](VIVI_SKILLS.md)를 사용한다. [비비 모션과 재생성](BASIC_ATTACK_MOTIONS.md). 인사는 기존 오른팔 동작으로 전환하고, 선택·춤·조준 중에는 무기와 팔을 함께 숨긴다. 데스캠 대상이 같은 캐릭터여도 팔의 원본 연결을 새 대상에 맞춘다.
- `cargo run --offline --bin tsb -- --weapon-review`는 전체 102개 GPU 화면을 검사한다. 접촉 오차 1mm 미만과 비비의 화면 좌우 배치를 검증하며, `--vivi-only`를 추가하면 비비 22개 화면만 실행한다. 초기 파지 자료는 `artifacts/weapon-hands/`, 쌍검 수정 화면·로그·변경 전 코드는 `artifacts/vivi-dual-blades/`에 보관한다.

## 나이아·에르핀·리뉴아 Tripo 적용 / 2026-11-13

- `assets/temp`의 사용자 ZIP 19개를 검사하고 `assets/character-sources/tripo/{naia,erpin,renua}/`로 정리했다. `rigged`, `hair`, `clothes`, `accessories`, `weapon` 등 용도별 이름을 사용하며 FBX 내부 텍스처 경로도 함께 고쳤다. 원본 ZIP은 각 캐릭터의 `archives/`에 바이트 그대로 보관한다. 원래 이름·해시·새 위치는 `source-files.json`, 대응 사양은 `scripts/tripo_roster_sources.json`에 있다.
- 나이아는 원본 본체·땋은 머리, 의상, 팔찌, 샌들과 돌고래 물총을 사용한다. `#20` 큰 모자는 `head-hat`로 머리에, `#45` 작은 꽃 모자는 `weapon-hat`로 돌고래 물총 위에 동시에 적용한다. 두 모자를 대안으로 취급했던 최초 조립을 원작 전신과 무기 일러스트에 맞춰 정정했다. 원본 자동 리깅에서 팔에 연결됐던 두 땋은 머리는 독립 메시 단위로 머리 본에 다시 연결했다.
- 에르핀은 본체에 긴 금발 머리, 왕실 의상, 망토, 날개, 왕관·장식을 조립하고 지팡이를 연결했다. 리뉴아는 흰 단발, 외투·부츠, 후광·귀걸이·술 장식을 조립하고 시계 마법진을 연결했다. 서로 다른 파츠의 정규화 크기와 소매 자세를 맞췄으며 불투명 의상 안의 몸 표면은 제거했다.
- 에르핀 왕관은 원작 전신 캡처와 `assets/character-inputs/inputs-masked/erpin.png`의 작은 왕관 비율에 맞춰 기존 폭·깊이의 62%, 높이의 52%로 줄였다. 보석과 금속 장식 1,108정점을 함께 변환하고 머리 위 접촉 높이를 조정했다. `import_tripo_roster.py`의 `fit_erpin_crown`이 재조립에도 같은 보정을 적용하며, 당시 일회성 보정은 기존 완성 모델의 왕관만 수정했다. Blend·FBX·미리보기·3D 초상화를 갱신했으며 변경 전 파일·정면/측면/후면 렌더·검증 결과는 `artifacts/erpin-crown-correction/`에 있다.
- 에르핀·리뉴아의 머리카락은 원작 전신과 개별 입력 일러스트를 기준으로 크기·위치와 두상 곡면을 다시 맞췄다. 두피는 보존하며 앞머리와 윗머리는 `Head`에 고정하고 아래로 늘어진 끝부분만 `Hair` 본으로 흔든다. 나이아의 큰 모자 역시 `Head`에 고정하고 작은 모자는 무기 메시와 합쳐 1·3인칭에서 같은 위치를 유지한다. 수정 후보·정면/측면/후면 렌더·변경 전 백업과 검증 로그는 `artifacts/tripo-roster-fitting/` 및 `artifacts/tripo-roster-import/{slug}/`에 있다.
- 소매·손목은 이후 별도로 정정했다. 세 캐릭터의 소매 단면 중심과 길이를 원본 팔·손목 본에 맞추고, 리뉴아의 내려간 소매는 입구를 납작하게 만드는 대신 회전해 T 자세에 맞춘다. 천의 긴 삼각형을 분할해 굽힘을 보완하고 소매 끝은 `Forearm.L/R`에 고정한다. 소매 안쪽 팔 표면을 보존해 사각 영역 삭제로 생기던 잘린 손목을 제거했다. 나이아 팔찌도 손목으로 옮겼다. 대기·공격·춤 확대 렌더와 이전 파일은 `artifacts/tripo-roster-sleeves/`에 보관하며, `--detail-review`는 반복적인 전체 렌더를 생략하고 손목 확대 렌더를 출력한다.
- 소매 정정 검증은 `artifacts/tripo-roster-sleeves/delivery-verification.json`에 있다. 양쪽 6개 소매의 손목 축·크기·전완 연결과 캐릭터별 180프레임, 전체 15명/165클립/60루프 검사를 통과했다. 게임 캡처 87장과 엔진 오류 0개, 원본 ZIP 19개 보존 및 다른 12명 FBX 해시를 확인했다. `wrist-comparison.png`는 Blender 확대 렌더의 수정 전후 비교다.
- 다리 연결도 일러스트와 대조해 정정했다. 나이아는 의상 파일에 포함된 회색 마네킹 다리·뒤판을 제거하고 원본 맨다리를 복원한다. 에르핀은 흰 스타킹을 신발 입구까지 이어 맞춘다. 리뉴아는 부츠를 실제 다리 축에 맞추고 골반에 잘못 연결된 가중치를 허벅지·정강이·발로 바꾼다. 반바지는 몸에 맞는 원본 형상에 제공된 남색 천 텍스처를 적용해 허벅지와 연결하며, 의상 안의 피부 삭제 경계는 입구 안쪽에 둔다.
- 다리 정정의 백업·대기/걷기/달리기 정면·측면 확대 렌더·수정 전후 비교는 `artifacts/tripo-roster-legs/`에 있다. `verify_tripo_legs.py`는 맨다리 표면, 스타킹/신발 및 반바지/허벅지 겹침, 부츠 입구 여유, 발바닥 가중치·모션 중 부착을 검사한다. 세 캐릭터 각각 11클립/180개 표본 프레임과 기존 머리·손목 검사, 전체 15명/165클립/60루프 검사 및 FBX 재가져오기를 통과했다. 적용 후 선택 30장·인게임 앞뒤 춤 30장, 엔진 오류 0개와 원본 ZIP 19개·다른 12명 모델 보존을 확인했다. 현재 적용본 검증은 `artifacts/tripo-roster-legs/delivery-verification.json`이다.
- 원본 손가락 리그를 보존하고 `Hand.R`, `UpperArm.R`, `Forearm.R` 등 게임 소켓 이름을 맞췄다. 기존 11종 동작을 원본 본 축으로 변환해 0–528프레임에 구웠고, T자 원본 팔은 대기 자세에서 내려오도록 보정했다. 얼굴은 제공된 고정 표정이며 페이셜 모프나 옷감 물리는 추가하지 않았다.
- 런타임은 `assets/characters/{slug}.fbx`와 `{slug}-weapon.fbx`를 사용한다. 새 무기 모델은 1인칭과 월드 캐릭터 양쪽에 적용된다. 춤·인사·선택 미리보기에서는 기존 규칙대로 무기를 숨긴다. 초상화·Blender 원본·모델 미리보기와 텍스처도 같은 폴더에서 교체한다.
- 재구성: Blender에서 `scripts/prepare_tripo_roster_sources.py`, `scripts/import_tripo_roster.py`, `scripts/verify_tripo_roster.py`, `scripts/publish_tripo_roster.py` 순서로 실행한 뒤 `python scripts/art_catalog.py`를 실행한다. 원본 준비 스크립트는 `assets/temp` 또는 보관한 ZIP에서 다시 추출할 수 있다. 작업 후보·검증·기존 3명과 코드 백업은 `artifacts/tripo-roster-import/`에 보존한다.

- 모자·머리카락 위치 정정 후 검증: 15명 리그·165클립·60반복 구간, 세 후보 FBX 재가져오기·가중치·변형 검사 통과. 세 캐릭터별 180개 표본 프레임에서 머리 부착 변위는 최대 0.000001m 미만이다. 최신 게임 캡처 111장(선택 30, 춤 30, HUD/무기 27, 인사 24)을 저장하고 대상 모델의 앞뒤·나이아 무기를 시각 확인했다. 리뷰 로그의 엔진 오류는 0개이며 원본 ZIP 19개 SHA256/CRC와 다른 12명 모델의 해시를 대조했다. 최신 전달 해시와 캡처 검증은 `artifacts/tripo-roster-fitting/delivery-verification.json`, 적용 화면은 `ingame-three.png`와 `naia-weapon-ingame.png`에 있다. 최초 코드 통합 시 fmt·Clippy·클라이언트 빌드도 통과했으며 이번 정정은 에셋과 제작 스크립트 변경이다.

## Tripo 재생성 입력·표정 사양 / 2026-11-12

- 현재 제출 자료는 [15명 전체 목록](../assets/character-inputs/tripo-roster/index.html), [개별 사용방법](../assets/character-inputs/tripo-individual/사용방법.txt), [전체 ZIP](../assets/character-inputs/tripo-roster-15.zip)이다. 버터를 포함한 15명 본체와 원작 기준 독립 무기·소품·효과를 제공하며, 현재 개수는 목록과 manifest에서 확인한다. 각 개별 ZIP에 PNG/프롬프트 짝, 사용방법, 리깅 TXT/JSON이 들어 있다.
- 눈 감기·가늘게/크게 뜨기, 눈썹, 입꼬리 올림/내림, 볼 부풀림/늘림/눌림을 좌우 독립 모프로 요청한다. 입 벌림/오므림/가로 늘림을 합쳐 캐릭터당 23개 모프 사양이며 아직 구현된 3D 데이터는 아니다. 귀·꼬리·머리카락은 캐릭터별 보조 본 체인으로 요청한다.
- 버터의 눈을 뜨고 입을 편하게 다문 본체를 새로 만들었다. 이드는 바디슈트를 유지하고 금색 눈을 뜬 본체를 사용한다. 아야는 발목 길이 흰 양갈래를 유지한 빈손 본체와 꽃잎 양산을 분리하며, 기존 양산을 든 그림은 조립 참고용이다. 이 최신 Tripo 입력 방식이 아래 과거 작업의 본체/탑승형 제출 안내를 대체한다.
- 클로에/세바스티안 구조를 사용자 첨부 두 그림과 캐릭터 자료로 다시 정정했다. 세바스티안은 후드/모자가 아니라 클로에 등 뒤에 있는 큰 곰 인형이며, 클로에는 양손으로 곰의 볼 아래를 받친다. 보라색 부분은 곰의 목 리본이다. Tripo 입력은 곰·보라 망토를 완전히 제외한 클로에 단독과, 머리·몸통·양팔·다리·리본을 갖춘 세바스티안 전신 두 개로 바꿨다. 두 리그를 별도로 만들고 CarryAnchor와 CheekSupport 손 IK 접촉점을 맞춘다. 이전 팔 단독 입력은 중복 조립을 막도록 `artifacts/tripo-chloe-sebastian-correction/retired/`로 옮겼다. 생성 기록·첨부 원본·조사·이전 자료도 같은 작업 폴더에 보존한다. 기존 게임 모델은 이 입력 자료 작업에서 재제작하지 않았다.
- 세바스티안의 외형은 팬 제작 첨부를 우선했던 오류를 다시 정정했다. [공식 기본 일러스트](https://x.com/trickcal_jp/status/1971582899796484364), [공식 캐릭터 소개](https://x.com/trickcal_jp/status/2058729771199316128), 원작 Normal/Idle_SebastianForm 전신을 대조해 양 볼의 산호색 하트 선무늬·라벤더 하트형 바보털·흰 몸통·보라 리본·옆 고리와 술·짧은 다리를 반영했다. 실사 봉제 질감·노란 막대·노출 금속 지퍼를 제거하고 2D 셀 채색 입력으로 교체했다. 하트/퀼팅 무늬는 모프 표면을 따르게 하고 바보털·술 보조 본을 사양에 추가했다. 원본·이전 파일·조사·생성 기록은 `artifacts/tripo-sebastian-original-style/`에 있다. 팬 첨부는 구조 설명용 이력이며 색·무늬·재질의 기준이 아니다.
- 원작 공격 모션과 전신 참고를 대조했다. 나이아의 기본 무기는 돌고래 물총이며 물줄기 효과는 별도 선택 입력이다. 처음 물 효과만 넣었던 누락을 Attack2_1 모션과 [공식 카드 공지](https://game.naver.com/lounge/Trickcal/board/detail/8404614)의 외형으로 정정했다. 카드 그림의 작은 꽃 밀짚모자는 분리 액세서리로 요청한다. 리뉴아의 시계형 마법진과 벨라의 도깨비불은 효과용 형태다. 마요는 침 발사관, 클로에는 별도 곰 인형 세바스티안 전신이다. 이드의 초승달 장치/로봇, 우이의 클로버/개구리를 각각 별도 입력한다.
- Tripo 자동 리깅만으로 지정된 모프·추가 본이 생성된다고 보장할 수 없다. 생성 결과 확인 후 Blender 등의 후속 리깅이 필요할 수 있고, Fyrox 모프 임포트/구동 및 보조 본 물리는 별도 통합 작업이다. 이 입력 자료 준비 단계에서는 게임 모델·모션을 변경하지 않았다. 이후 실제 적용 현황은 문서 첫머리의 2026-11-13 항목을 따른다.
- 재구성: `python scripts/prepare_tripo_roster.py`. 원본 사양은 `scripts/tripo_input_specs.json`, 공통 생성/리깅 문구는 `scripts/tripo_package.py`다. 생성 원본·조사·이전 자료·검증은 `artifacts/tripo-animation-inputs/`에 보관한다. 이전 14명 ZIP은 활성 제출 폴더에서 이력 폴더로 옮겼다.

## 피코라 숄더백 누락 정정 / 2026-11-12

- 사용자 첨부에 더해 [원작 캐릭터 아이콘](https://cheeky.tsuncloud.com/deploy/com.epidgames.trickcalrevive/sprites/rawsprites/heroicons/picora/Picora.png)과 [원작 전투 아틀라스 파츠 목록](https://cheeky.tsuncloud.com/deploy/com.epidgames.trickcalrevive/spines/spine/ingame/hero/picora/picora/skeleton.atlas)을 확인했다. 커뮤니티가 호스팅하는 원작 자료이며, `Bag`, `Bag_Ribbon`, `Bag_Stciker`, `Bag_Strip`, `Bag_Strip_B`가 별도 파츠로 존재한다. 첨부의 출처는 별도로 단정하지 않는다.
- 크림색 타원 가방·분홍 곡선 무늬와 옆면·흰 상단 장식·분홍 리본·긴 흰 끈·체리 장식을 반영했다. 끈은 같은 쪽 어깨 앞뒤로 이어지고 가방은 같은 쪽 골반 옆에 놓인다. 정면 화면 오른쪽을 캐릭터 L로 설정했으며 가슴 대각선을 가로지르는 크로스백으로 만들지 않는다.
- 내장 image_gen으로 본체, 가방 단독, 착용 정면, 착용 삼면도 4장을 작성했다. 본체에서 가방 끈의 체리 장식을 제거해 중복을 막고, 중립 입·격자 치마·줄무늬 쪽 신발 장식을 정리했다. `front` / `weapon` / `prop-bag`를 각각 입력하며, `assembly-reference` / `turnaround-reference`는 조립 참고용이다.
- [피코라 개별 ZIP](../assets/character-inputs/tripo-individual/피코라-picora.zip), 프롬프트·리깅 TXT/JSON, 전체 목록·manifest·ZIP 및 `assets/turnarounds/picora.png`를 갱신했다. `BagShoulder.L` 소켓, `BagRoot`, 앞뒤 끈 체인과 체리 체인의 연결·관통 검사 사양을 추가했다. 실제 3D 리그/게임 모델은 이 작업의 결과에 포함되지 않는다.
- 생성 원본·실제 프롬프트·추가 조사·이전 파일·검증은 `artifacts/tripo-picora-shoulder-bag/`에 보존한다. 미확인 후면·두께·연결부는 제작용 해석이다. 이전 `inputs`, `inputs-masked`, `prepared-preview`의 잘라내기 자료는 이력이며, 현재 Tripo 입력은 `scripts/tripo_input_specs.json`이 지정하는 새 본체를 사용한다.

## 이드 민소매 바디슈트 외형 정정 / 2026-11-12

- 사용자 최종 정정과 첨부 이미지 2장을 복장 기준으로 삼았다. 흰 중앙 패널·검정 옆면의 몸에 붙는 원피스 바디슈트, 흰 목 밴드와 검정 교차 스트랩, 맨팔·흰 손목 밴드, 짙은 허벅지 길이 스타킹·흰 옆선·흰 낮은 신발을 반영했다. 앞서 해석한 트레이닝복은 폐기한 작업 단계다.
- 내장 image_gen으로 본체 A포즈, 탑승형 정면·좌측·우측·후면 및 삼면도 6장을 수정했다. 사용자 첨부 자료·생성 원본·프롬프트·검증 기록은 `artifacts/id-bodysuit-correction/`에 보관한다. 첨부 자료를 공식 이미지로 표기하지 않으며, 측후면과 가려진 세부는 팬게임 제작용 해석이다.
- Tripo 개별 PNG·TXT·ZIP, 14명 공용 패키지, 탑승형 자료, 입력 PNG 3종, 직접 다중시점 입력과 삼면도·미리보기·메타데이터를 갱신했다. `tripo-individual`에는 버터를 포함한 15명 파일을 유지한다. 작업 중간 파일은 `character-inputs` 안에 별도 폴더로 추가하지 않는다.
- 절차 제작 모델의 옷을 같은 바디슈트·스타킹 구성으로 바꾸고, 초승달 장치·로봇과 기존 20본·11동작을 유지했다. 11,396정점·22,356삼각형이다. Blender·FBX·내장 삼면도·초상화·미리보기를 갱신했다. 다른 14명 Blend·FBX 28개는 변경 전 SHA256과 일치한다.
- 당시 절차 제작 모델은 `correct_id_assets.py`와 `build_characters.py`에 전용 20본 검증·카탈로그 갱신을 적용했다. 현재 모델 재생성에는 이 구형 절차를 사용하지 않는다. 이전 로브/트레이닝복 자료는 각각 `artifacts/id-tracksuit-correction/before/`, `artifacts/id-bodysuit-correction/before/`에 작업 이력으로 보존한다.
- 검증: 15명 리그·165클립·반복 구간과 가중치 검사 통과. 선택 30장, 앞뒤 춤 30장, HUD 큰/작은 화면 각 27장, 커뮤니케이션 24장의 GPU 캡처를 갱신하고 PNG 크기를 확인했다. 이드 선택·춤·HUD 초상화·1인칭 맨팔을 시각 확인했다. 개별 ZIP 2파일과 공용 ZIP의 이드 파일이 현재 PNG·TXT와 일치한다. 상세 결과는 `delivery-verification.json`, `runtime-verification.json`, `model-validation.json`에 있다.

## 아야 장발 양갈래·양산 외형 정정 / 2026-11-12

- [아야 원작 이미지](https://wikiwiki.jp/thetrickal/%E3%82%A2%E3%83%A4)와 기존 Spine 캡처를 재검토했다. 아야는 **발목까지 내려오는 흰 장발 양갈래**다. 귀 아래 양쪽 나비 장식에서 이어지는 두 갈래를 기모노 옷자락과 구분한다. 머리 위 큰 흰 꽃잎은 손잡이를 쥔 양산이다. 앞선 양산 수정 때 머리를 단발로 기록한 것은 오해였으며 이번 자료로 정정한다.
- 내장 image_gen으로 정면·좌측·우측·후면·삼면도를 다시 만들었다. 작업용 원본과 실제 생성 프롬프트는 `artifacts/aya-twintail-correction/images/`에 보관한다. Tripo 제출 파일은 `assets/character-inputs/tripo-individual/`에 있다. 측·후면의 미확인 세부는 팬게임 제작용 해석이다.
- 입력 PNG 3종, 두 다중시점 폴더, Tripo 목록·프롬프트·개별 파일·개별/전체 ZIP과 모음 미리보기를 갱신했다. 아야 프롬프트에 `ANKLE-LENGTH WHITE LOW TWIN TAILS`와 두 갈래의 연결·분리 조건을 명시했다. 양산을 든 자세를 유지하고, 빈손 A포즈를 강제하지 않는다.
- 절차 제작 Blender/FBX에 발목 높이까지 이어지는 두 갈래를 만들고 Hair 본에 연결했다. 손에 가중치를 둔 꽃잎 양산을 유지한다. 20본·11동작, 17,484정점·34,504삼각형이다. 초상화와 모델 미리보기, 내장 삼면도도 갱신했다. 앞서 적용한 양산 중복 표시 방지·1인칭 양산·표시명·선택 화면 크기는 유지한다.
- 이번 변경 전 자료는 `artifacts/aya-twintail-correction/before/`에 보관했다. 원작 참고 원본과 과거 검증/백업은 기록으로 보존한다. 이전 양산 수정 기록은 `artifacts/aya-parasol-correction/`, 현재 양갈래 수정 근거는 `artifacts/aya-twintail-correction/`에 있다.

## 버터 Tripo 모델 적용 / 2026-11-12

- 사용자 제공 파일은 본체 리깅 FBX, 의상 메시 FBX, 액세서리 메시 FBX이며 각각 4096×4096 색상 텍스처가 있다. 의상·액세서리에는 원본 리깅이 없어 조립 후 본체에 연결했다.
- 원본 이름은 `assets/character-sources/tripo/butter/`의 `rigged/butter-rigged.fbx`, `clothes/butter-clothes.fbx`, `accessories/butter-accessories.fbx`로 정리했다. 각 `.fbm` 폴더의 텍스처 이름과 FBX 내부 경로도 함께 수정했다. 변경 전 파일과 기존 게임용 버터는 `artifacts/tripo-backups/20261112-192408/`에 보존했다. 원본 대응표는 `source-files.json`에 있다.
- 세 파츠의 개별 정규화 크기를 몸에 맞췄다. 본체의 Tripo 가중치·손가락 본은 유지하고 의상과 장식의 가중치를 추가했다. 불투명 의상 안에 가려지는 몸 표면을 제거해 관통을 줄였다. 원본 FBX는 이 조립 편집의 영향을 받지 않는다.
- 11개 기존 게임 동작을 Tripo 관절 축으로 변환해 구웠다. 게임의 `Hand.R` 무기 소켓과 0–528프레임 구간을 유지한다. 캐릭터 선택 초상화와 미리보기도 새 모델로 교체했다. 얼굴은 제공된 모델의 고정 표정이다.
- 적용 경로는 `assets/characters/butter.blend`, `butter.fbx`, `butter-portrait.png`, `butter-preview.png`와 같은 폴더의 `butter-*-basecolor.jpg`다. Fyrox의 기본 `RecursiveUp` 검색은 하위 폴더를 탐색하지 않으므로 게임 텍스처는 FBX 옆에 둔다.
- 재조립은 Blender에서 `scripts/import_tripo_butter.py`, 후보 검증은 `scripts/verify_tripo_butter.py`, 적용은 `scripts/publish_tripo_butter.py` 순서로 실행한다. 그 뒤 `python scripts/art_catalog.py`로 해시·목록을 갱신한다. 후보와 변형 검증은 `artifacts/tripo-butter/`에 저장한다.
- 최종 검증: 버터 15,336정점·28,854삼각형·70본, 11클립과 4개 반복 구간, 가중치 합·정점당 최대 4본, FBX 재가져오기 통과. 표본 프레임의 최악 p99 edge 길이비는 1.206, 2배 초과 edge 비율은 최대 0.192%다. 15명 전체 165클립 검사, Fyrox 앞뒤 춤 30장과 선택 화면 30장 캡처를 완료했고 버터의 세 텍스처 로드 및 실제 화면을 확인했다. 다른 14개 FBX는 적용 전 SHA256과 일치한다.

## 저장 위치

| 경로 | 내용 |
|---|---|
| assets/references/manifest.json | 15명 원작 이미지/모션 URL, 자료 종류, 날짜, 해시 |
| assets/references/*-fullbody.png | 공개 원작 모션 뷰어의 전신 캡처 |
| assets/references/*-motion-notes.txt | 뷰어에서 확인한 동작 이름과 프레임/시간 목록 |
| assets/references/*-motion-reference.skel | 원작 2D 동작 조사용 파일 |
| assets/turnarounds/*.png | 캐릭터별 정면·좌측면·후면 가이드 15장 |
| assets/turnarounds/generation.json | 내장 image_gen 모드, 실제 15개 프롬프트, 생성 원본 경로 |
| assets/character-inputs/ | 보존한 정면 잘라내기·배경 제거 이미지, 방향별 입력과 원본 좌표 기록 |
| assets/characters/*.blend | 편집 가능한 메시·리그·가중치·동작·내장 삼면도 (버터 70본, 나이아·에르핀·리뉴아 72본, 나머지 20본) |
| assets/characters/*.fbx | Fyrox가 불러오는 15개 모델과 애니메이션 |
| assets/characters/*-preview.png | Blender 렌더 15장 |
| assets/characters/manifest.json | ID·에셋 경로·정점/삼각형 수·클립 구간·해시 |
| artifacts/character-art-verification.json | 15개 리그·165개 동작·루프·가중치·크기 검사 |
| artifacts/preview-gallery-*.png | 실제 Fyrox에서 15명 춤의 앞/뒤를 캡처한 30장 |
| artifacts/character-model-contact.png | Blender 15명 렌더 모음 |
| artifacts/character-ingame-contact.png | Fyrox 15명 춤 전면 모음 |

참고 WebP thumbnail은 2D 텍스처 아틀라스다. 조립된 전신 이미지는 fullbody 캡처를 사용했다. 런타임은 새 FBX만 불러오며 references 자료를 텍스처나 원본 리그로 사용하지 않는다.

## 캐릭터별 조사 출처와 반영 요소

원작 디자인과 2D 동작의 제작자는 EPID Games, 공개 자료 호스트는 Cheeky 커뮤니티 뷰어다. 모든 URL은 2026-11-12 확인했다. 뷰어에는 Idle / Move / Attack / Skill / Ultimate·UserSkill / Victory 등의 원작 동작과 재생 시간이 있다. 게임용 3D 동작은 이 자료를 참고해 30fps로 새로 작성했다.

| ID | 캐릭터·자료 | 모델에 반영한 구분 요소 |
|---|---|---|
| 0 | [비비](https://wiki.cheeky.tsuncloud.com/korea/spines/vivi) | 흰 장발, 뿔, 용 날개 장식, 수은검, 흰 겉옷·버건디 드레스, 부유 |
| 1 | [나이아](https://wiki.cheeky.tsuncloud.com/korea/spines/naia) | 푸른 머리·양갈래 땋기, 꽃 밀짚모자, 물빛 의상 |
| 2 | [에르핀](https://wiki.cheeky.tsuncloud.com/korea/spines/erpin) | 금발, 왕관, 흰 왕실 의상, 붉은 망토, 날개·지팡이 |
| 3 | [이드](https://wiki.cheeky.tsuncloud.com/korea/spines/ed) | 졸린 흰머리 엘프, 흰색·검정 민소매 바디슈트·교차 목 스트랩·짙은 스타킹, 앉은 자세, 부유 초승달 장치, 동반 로봇 |
| 4 | [아야](https://wiki.cheeky.tsuncloud.com/korea/spines/aya) | 갈색 피부, 발목 길이 흰 장발 양갈래, 양쪽 나비 묶음, 흰 기모노·검은 띠, 손잡이를 쥔 흰 꽃잎 양산 |
| 5 | [피코라](https://wiki.cheeky.tsuncloud.com/korea/spines/picora) | 청록 사탕 머리, 양쪽 번, 분홍 상의·청록 격자 치마, 사탕 지팡이; Tripo 입력에는 흰 끈·체리 장식의 크림색 숄더백 추가 |
| 6 | [클로에](https://wiki.cheeky.tsuncloud.com/korea/spines/chloe) | 금발 높은 양갈래·황색/흰색 의상인 클로에와, 등 뒤의 별도 곰 인형 세바스티안 |
| 7 | [리뉴아](https://wiki.cheeky.tsuncloud.com/korea/spines/renewaawaken) | 플레이어블 각성 버전, 흰 단발 위 양쪽 이어폰·세로 장식, 머리 뒤 위쪽에 떠 있는 기하학 후광, 남색 외투·부츠 |
| 8 | [마요](https://wiki.cheeky.tsuncloud.com/korea/spines/mayo) | 보랏빛 갈색 포니테일, 파란 모자·앞치마, 수집 가방·요정 날개 |
| 9 | [벨라](https://wiki.cheeky.tsuncloud.com/korea/spines/vela) | 은백색 머리, 남색 유령 드레스, 도깨비불·부유 리본 |
| 10 | [우이](https://wiki.cheeky.tsuncloud.com/korea/spines/ui) | 민트 땋은 머리, 노란 우비, 개구리 모자와 큰 개구리 |
| 11 | [버터](https://wiki.cheeky.tsuncloud.com/korea/spines/butter) | 늘어진 강아지 귀·꼬리, 노란 리본·의상, 줄무늬 양말·새총 |
| 12 | [코미](https://wiki.cheeky.tsuncloud.com/korea/spines/kommy) | 흰 장발·분홍 끝, 삼각 고양이 귀·꼬리, 잠옷·베개 |
| 13 | [힐데](https://wiki.cheeky.tsuncloud.com/korea/spines/hilde) | 검은 포니테일, 안경, 크림색 니트·가운·명찰, 주사기총 |
| 14 | [시온](https://wiki.cheeky.tsuncloud.com/korea/spines/xxionx) | 남색 포니테일, 옆 용 가면, 서로 다른 눈 색, 어두운 망토·저격총 |

보조 조사: [GAGETHER 캐릭터 도감](https://gagether.wiki/ko/games/trickcal/characters/erpin), [에르핀 스킬 GIF 게시물](https://m.ruliweb.com/community/board/300143/read/64579519), [피코라 공식 소개](https://game.naver.com/lounge/Trickcal/board/detail/3615706), [아야 개편 공지 자료](https://www.newri.net/user_news/223492?listStyle=viewer&order_type=desc&page=12&sort_index=title).

## 동작 구간

Blender의 TSB_Motions 액션에 아래 구간이 들어 있다. Fyrox는 이 구간만 잘라 재생한다. 단위는 30fps 프레임이며 원작 전투 판정 시간은 서버 키트가 결정한다.

| 동작 | 프레임 | 반복 | 게임 조건 |
|---|---|---|---|
| Idle | 0–60 | 예 | 정지 |
| Walk | 63–93 | 예 | 보행; 부유 캐릭터는 부유 이동 |
| Run | 96–120 | 예 | 질주 속도 |
| Jump | 123–153 | 아니오 | 공중 |
| Attack | 156–180 | 아니오 | 서버 기본공격 발생 |
| Skill | 183–219 | 아니오 | E 발동 |
| Ultimate | 222–282 | 아니오 | Q 발동 |
| Dance | 285–405 (정규화) | 예 | 서버 춤 상태; 원본 확인 5명은 1.067–2.667초, 나머지는 기존 4초 |
| Hurt | 408–420 | 아니오 | 피격 |
| Death | 423–453 (정규화) | 아니오 | 사망 직후, 사도별 1.833–3초 후 마지막 자세 유지 |
| Select | 456–528 | 아니오 | 선택창 미리보기 진입·캐릭터 변경; 이후 Idle |

원본 춤이 미확인인 10명은 팔·상체 흔들기와 발 동작을 조합한 기존 제작 루프를 사용한다. 3가지 팔/머리 변형과 캐릭터별 머리카락·망토·꼬리·탑승물 움직임을 사용한다. 이드는 앉은 자세를 유지하며 초승달 장치가 기울고, 우이는 개구리도 함께 움직인다. 표정은 고정이며 의상과 장식은 단순화한 MVP 메시다. 손가락 리깅·페이셜·의상 물리·립싱크·음악은 포함하지 않는다. 삼면도의 측면·후면과 일부 의상 연결은 팬게임 제작 해석이다.

## 춤 관찰과 네트워크

- G를 새로 누르면 춤 시작/종료. 로비에서도 선택 화면을 닫고 마우스를 잡으면 사용할 수 있다.
- 춤 중 마우스는 몸 주위를 360도 회전, 상하 각도 제한, 휠은 1.8~6.5m 줌. 벽·엄폐와 바닥 안으로 카메라가 들어가지 않도록 거리를 줄인다.
- 카메라만 회전하므로 서버 조준 방향은 유지된다. 종료하면 기존 1인칭 조준으로 복귀한다.
- 이동·질주·점프·기본공격·E·Q·재장전·피격·사망·캐릭터 교체·라운드 전환·봇 인계가 춤을 취소한다. 춤 자체는 무적이나 이동 능력을 부여하지 않는다.
- TSB4의 actor 상태 비트와 입력 비트로 전송한다. 기존 패킷 크기를 유지한다. 서버 tick 기반 춤 위상으로 관전자와 늦은 참가자도 같은 동작을 본다.

## 재생성·검증

리뉴아 머리 장식은 `import_tripo_roster.py`의 `fit_renua_head_accessories`에서 맞춘다. 이어폰과 세로 장식은 머리카락 바깥에 배치하고, 후광 전체는 머리 뒤로 분리해 게임 카메라에서도 윗부분이 드러나게 했다. 당시 일회성 머리 장식 보정은 기존 메시의 머리 장식 정점만 변경하고 리그·UV·동작 키를 보존했다. Tripo 후보 검사를 거쳐 반영한 교체 전 모델의 기록이다. 후광을 포함한 전체 높이 제한은 2.8m이며 몸 크기와 충돌 판정은 그대로다.

리뉴아의 외부 시점에서는 FBX에 포함된 머리 뒤 후광을 사용하고 손 옆의 별도 원형 무기는 숨긴다. 1인칭 공격 표현은 유지한다. 선택창은 리뉴아만 0.85배로 표시해 선택 모션 중에도 후광이 화면 안에 들어오게 한다. `--weapon-review --renua-only --auto-start`로 1인칭 대기·공격·재장전 및 외부 시점 대기·공격의 중복 무기 숨김을 확인할 수 있다.

프로젝트 루트에서 실행한다. 이미 생성된 FBX를 사용할 때는 Blender가 필요 없다.

```powershell
# 게시된 리그/클립 검증
$blender = "$env:LOCALAPPDATA/Microsoft/WindowsApps/blender-launcher.exe"
& $blender --background --factory-startup --python-exit-code 1 --python scripts/verify_character_art.py
python scripts/art_catalog.py
cargo run --bin tsb -- --art-review --auto-start
cargo run --bin tsb -- --selection-review
pwsh -File scripts/art_contact_sheets.ps1
pwsh -File scripts/verify.ps1 -Smoke
```

`build_characters.py`는 절차 제작 모델의 생성 코드이며, 선택 모션은 `add_selection_motions.py`에서 추가했다. 기존 에셋을 검증된 이전 백업에서 복원한 뒤 아야·이드 외형을 해당 생성 코드로 수정했다. 버터는 사용자 제공 Tripo 모델을 적용했다. 기본 모델·모션 템플릿은 `artifacts/charactergen-original-rigs`에 보존했다.

## CharacterGen 생성 모델 제거 및 이전 모델 복원

- 공용 설치 `C:/Users/user/Tool/Work/CharacterGen` 전체를 삭제했다. 전용 Python 환경·AI 가중치·캐시·로컬 CUDA 도구와 생성 작업 폴더가 포함된다. 시스템 Python·Blender·그래픽 드라이버는 유지했다.
- 사용자 요청에 따라 CharacterGen 생성 모델, 후보·테스트 출력, 생성 텍스처와 전용 생성·가져오기 스크립트를 제거했다.
- 15명 모두 적용 전 백업에서 Blend·FBX·초상화·미리보기를 복원했다. 복원 전 백업 해시와 복원 후 파일 해시를 대조했다. `artifacts/charactergen-backups`와 `artifacts/charactergen-original-rigs`에 남은 모델은 CharacterGen 이전 원본이다.
- 원작 참고자료와 image_gen 삼면도는 보존했다. 설치 폴더에 있던 정면 입력 15장, 배경 제거 입력 15장, 전처리 미리보기와 방향별 잘라내기 입력은 `assets/character-inputs`에 보존했다.
- `assets/character-inputs/inputs/manifest.json`과 `input-plan.json`에 원본 경로와 자른 좌표가 있다. 보존한 JSON의 예전 설치 경로는 당시 작업 기록이며, 실제 이미지의 현재 위치는 `assets/character-inputs` 아래다.
- 제거·복원 목록은 `artifacts/charactergen-removal.json`을 참조한다. 과거 CharacterGen 실험 기록은 `WORK_LOG.md`에 남겨 두었으며 현재 사용 모델을 나타내지 않는다.
- 복원 검증: Blender에서 15개 리그·165개 클립·가중치 검사 통과. Fyrox에서 15개 FBX의 앞뒤 춤 30장과 선택창·선택 모션 30장 캡처 통과. 아트 목록과 모델/인게임 모음 이미지를 복원본으로 갱신했다.

춤 구현 당시 검증: 65개 자동 테스트·fmt·Clippy·클라이언트/서버 빌드·단독 그래픽 모두 통과 (`artifacts/verification/20261112-052430.log`). 실제 호스트 아야/참가자 버터 양쪽 춤·관찰 회전·이동 취소도 통과했다 (`artifacts/art-host.log`, `artifacts/art-client.log`). 15명 전체 앞/뒤 GPU 캡처는 `artifacts/art-gallery.log`에서 확인한다. 별도 PC와 인터넷 연결은 아직 검증하지 않았다.

선택창 확장: 15명 각각 고유 Select 동작과 초상화를 추가했다. 현재 총 165개 동작이며 상세 제스처와 검증은 [영웅 선택 사양](HERO_SELECTION.md)에 기록한다.

과거 CharacterGen 적용 당시 검증은 `artifacts/verification/20261112-171916.log`에 남아 있다. 현재 모델은 그 이후 이전 절차 제작본으로 복원되었으므로 해당 기록을 현재 에셋의 검증으로 해석하지 않는다.
