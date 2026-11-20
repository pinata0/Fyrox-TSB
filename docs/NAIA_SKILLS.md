# 나이아 스킬

고학년 Q는 보관된 `assets/references/naia-motion-reference.skel`의
`Ultimate1_1`(121/30초)을 사용한다. 몸, 양손, 머리, 땋은 머리 동작을
샘플링하고 기존 FBX 타임라인 675–796프레임에 추가했다.

## 효과

- 발동 1.5초에 시전자 뒤 4.5m에서 파도가 생성된다.
- 파도는 폭 10m, 높이 최대 3.8m이며 초속 10m로 3.2초간 전진한다.
- 닿은 아군과 시전자는 보호막 100을 5초간 얻는다. 체력 회복은 하지 않는다.
- 적에게 기본 피해 25와 2m 밀쳐내기를 한 번 적용한다. 기존 피해 보정,
  용족 밀쳐내기 저항, 지형 충돌과 무적 규칙을 따른다.
- 파도 중앙이 벽에 닿으면 종료된다. 각 대상의 진행 경로도 검사하여
  옆쪽 엄폐물 뒤의 대상에게 효과가 적용되지 않는다.
- 고학년 시전 중 이동·점프·일반 공격·재장전·저학년 스킬이 제한된다.
  파도 생성 전 행동 불가 상태가 되면 시전이 취소된다.

수치는 `src/naia_skill.rs`의 상수에서 조절한다. 원래의 저학년 회복 물줄기와
마우스 해제 시 이동 효과는 유지한다.

## 고래와 시점

`naia-weapon.blend`의 원래 총 메시, UV, 재질을 유지한 `swimmingWhale`
변형으로 몸을 넓히고 꼬리를 편다. 저학년 시작/종료에서 총과 고래 사이를
보간하고, 총구를 정면으로 유지한 채 양손 접점과 IK로 몸통의 양옆을 잡는다.
고학년은 무기를 숨기고 맨손으로 동작하며, 동일한 변형 메시 여섯 마리를
양옆에 소환한다. 원본 세 고래의 높이와 회전 샘플을 양쪽에 적용한다.
1인칭, 관전자, 사망 재생은 같은 스킬 시계를 사용한다.

저학년 E의 1인칭은 돌고래·손·소매에 공통 1.65배 확대를 적용하고,
자세를 0.28m 내려 소매 절단면이 화면 아래로 벗어나게 한다. 팔 전체 대신
손과 소매만 표시하며, 손의 1.0→1.9배 보간에 공통 배율을 곱한다.
시작·종료의 변형 시간에 맞춰 손 크기를 보간하고,
손바닥 접점을 몸통 좌우에 고정하고 양 손바닥을 안쪽으로 향하게 한다.
총구와 물줄기는 조준 방향을 향하고 외부 시점의 크기는 유지한다.
짧은 소매는 원본 옷감/테두리 재질과 리그에서
생성한 닫힌 메시이며 `src/presentation/first_person/naia.rs`에서 조절한다.
일반 자세·고학년 Q로 돌아오면 기존 손 크기와 소매를 복원한다.

## 별도 파도 에셋

- `assets/effects/naia-ultimate-wave-concept.png`: 내장 image_gen으로 만든 시안
- `assets/effects/naia-ultimate-wave-prompt.txt`: 생성 프롬프트와 파일 목록
- `assets/effects/naia-ultimate-wave.blend`: 수정 가능한 독립 모델
- `assets/effects/naia-ultimate-wave.glb`: 교환용 모델
- `assets/effects/naia-ultimate-wave-mesh.json`: 게임에서 공유하는 물/포말 메시
- `src/presentation/naia_wave.shader`: 흐름, 물결, 투명도와 포말

## 재생성과 확인

1. `node scripts/extract_naia_skill.cjs`
2. Blender에서 `scripts/bake_naia_skill.py` 실행
3. Blender에서 `scripts/build_naia_whale.py` 실행
4. Blender에서 `scripts/export_meshy_morphs.py -- assets/characters naia-weapon` 실행
5. Blender에서 `scripts/model_naia_wave.py` 실행
6. `cargo test --offline --no-default-features --test naia_skill --test combat`
7. `cargo test --offline --no-default-features --lib`
8. `cargo build --offline --bin tsb`
9. `target/debug/tsb.exe --solo --naia-skill-review`

렌더링 검증은 손이 가장 높아지는 시점을 포함한 저학년 10개·복귀 1개·고학년 5개 시점을 각각 외부 시점과
1인칭으로 캡처한다. 양손 접점, 돌고래·손·소매 확대, 소매 절단면 전체의
화면 이탈과 정면 총구,
일반 자세 복귀, 고학년 무기 숨김, 소환 고래 여섯 마리와 물줄기 표시를
실제 그래프에서 검사한다. 결과는 `artifacts/preview-naia-skill-*`에 저장된다.

나이아 단계와 시간은 기존 스냅샷 필드를 공유한다. 파도 방향은 투사체의
남는 비트에 저장하여 최대 96개 투사체 패킷의 크기를 유지한다.
프로토콜 식별자는 `TSBP`이므로 서버와 클라이언트 모두 같은 빌드를 사용한다.
