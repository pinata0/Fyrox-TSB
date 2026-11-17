# 아야 양산 변형 / 2026-11-15

아야의 기존 세 번째 강화 평타에 `Attack2_1`을 연결했다. 일반 평타 두 번에는 `Attack1_1`의 곡선 검을, 강화 평타에는 `Blade_2`에서 `Blade_3`으로 바뀌는 긴 날·언월도 형태를 표시한다. 공격 종료·다른 동작·캐릭터 전환 시 열린 양산으로 복귀한다.

## 모델과 원작 근거

- 기존 `aya-weapon.blend`의 5,965정점 양산, UV와 흰 꽃잎 재질을 보존했다. 실제 손잡이 중심을 원점에 맞추고 축을 세웠다.
- 기존 메시의 `parasolFold`와 `parasolSpread` 모프로 꽃잎을 연속해서 접고 모은다. 별도 입체 메시 `Aya_Blade_1/2/3`은 두께·날 면·둥근 외곽을 가진다. 칼날은 손잡이 쪽에서 펴지며 나타나고 원작의 불투명도 곡선을 출현 비율로 사용한다.
- 보관된 전투 Spine 원본의 꽃잎 `Weapon_Petal_01_0`, `Weapon_Petal_Root`와 Sword 슬롯을 30Hz로 평가했다. 일반 공격은 1.5초, 강화 공격은 88/30초이며 강화 공격의 칼날 교체는 43/30초다. 샘플은 `assets/characters/attack/aya-transform.json`, 출처 해시·관절 매핑·참고 렌더는 `artifacts/aya-transform/reference/`에 있다.
- 이 파츠는 3D로 새로 만든 형태다. 원작의 2D 무기 텍스처·잔상 스프라이트를 게임에 붙이지 않는다. 꽃잎은 메시 모프로 접히며 실제 기계식 우산의 살대 관절을 시뮬레이션하지 않는다.

## 게임 연결

전신 FBX의 일반 공격은 156–180프레임, 강화 공격은 531–555프레임이다. 전신/1인칭/녹화 데스캠은 같은 공격 시계와 무기 변형 샘플을 사용한다. 기존 게임 간격에 맞춰 두 동작 모두 0.658초로 재생하며 공격 속도 효과도 함께 적용된다. 기존 피해량·감속·공격 판정 시점은 유지한다.

서버가 실제 발동한 아야 공격의 단계를 `aya_combo`로 저장한다. 빗나가도 진행하며 쿨타임·기절로 거절된 입력은 진행하지 않는다. 비비와 캐릭터별로 동일한 2비트 네트워크 공간을 사용해 패킷 크기를 유지하고, 프로토콜은 `TSBJ`로 갱신했다. 멀티플레이 참여자는 같은 빌드가 필요하다.

FBX의 정점 중복 제거에 따른 모프 오프셋 문제는 기존 `morph_import` 경로로 복원한다. `aya-weapon.morphs.json`을 FBX와 함께 배포해야 한다. 원본 캐릭터 메시 7개·가중치·다른 동작 키는 보존 검사한다. 공격 구간 Euler 키는 연속된 회전 표현으로 맞춰 프레임 사이 회전 반전을 방지한다.

## 재생성과 검증

1. `node scripts/extract_aya_attack.cjs`
2. Blender 배경 실행으로 `scripts/build_aya_transform.py`
3. Blender에서 `scripts/export_meshy_morphs.py -- assets/characters aya-weapon`
4. Blender에서 `scripts/bake_aya_attacks.py`
5. `cargo run --bin tsb -- --solo --attack-review --aya-only`

무기 빌드 입력은 `artifacts/aya-transform/before/assets/characters/aya-weapon.blend`다. 편집 파일에는 일반 공격 0–45프레임과 강화 공격 90–178프레임의 변형 미리보기가 들어 있다. `scripts/render_aya_transform.py`로 형태별 렌더를 만든다.

전용 GPU 검증은 두 공격 × 전신/1인칭 × 8시점, 총 32장을 생성한다. 펼침·접히는 중간·칼날 교체·복귀 상태, 모프 비율, 칼날 표시, 전신 관절과 손잡이 접촉 오차 1mm 미만을 검사한다. 규칙 테스트는 강화 단계·패킷 누락·데스캠의 기록 보존과 무기 초기화를 검사한다.

검증 결과: 규칙·UDP 통합 테스트 219개와 데스크톱 테스트 46개 통과(기존 테스트 1개 ignored), 전체 타깃 Clippy 경고 0개, 포맷 검사와 릴리스 빌드 통과. GPU 32장 모두 통과하고 실제 칼날 크기도 검사했다. `scripts/verify_aya_transform.py`는 기존 UV/토폴로지 보존, 닫힌 칼날 메시의 바깥 방향 노멀, FBX 재가져오기 후 삼각형·모프 보존과 복귀를 확인한다. 결과와 로그는 `artifacts/aya-transform/`에 있다.

배포 파일: `release/TSB-Multiplayer-AyaTransform.exe`. 패키지 추출/체크섬 검증 및 이 EXE에서 직접 실행한 GPU 32장 검사도 종료 코드 0으로 완료했다.
