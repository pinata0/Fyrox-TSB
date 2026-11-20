# 리뉴아 모션과 시간 마법

보관된 `assets/references/renua-motion-reference.skel`을 Spine 런타임으로
30Hz 샘플링했다. 원본의 IK, 보간 곡선, 몸 이동, 손, 머리카락, 망토,
헤일로, 구조물별 회전·크기·투명도를 사용한다.

| 입력 | 원본 모션 | 동작 |
| --- | --- | --- |
| 좌클릭 | Attack1_1, 2.70초 | 1.30초에 푸른 에너지볼 1발, 기본 피해 15 |
| 우클릭 | Attack2_1, 2.87초 | 약 1.37초에 손에서 레이저 1발, 기본 피해 26. 다시 누르면 재사용 |
| E | Skill1_1, 4.97초 | 1.1 / 1.6 / 2.1 / 2.6 / 3.6초에 레이저. 각 기본 피해 12, 유효 적중 시 자신 HP 9 회복 |
| Q | Ultimate1_1, 16.03초 | 약 4.23초에 앞쪽 직사각형 시간 영역 생성 |

평상시와 모든 시전 중 맨손이다. 기존의 손에 붙은 마법진 소품과
네 번째 기본공격 자동 레이저는 사용하지 않는다. 일반공격도 원본 전체 길이로
재생하며 탄약을 소비하지 않는다. 수치는 `src/renua_skill.rs`에서 조절한다.
E 시전 중 이동과 조준이 가능하다. 일반공격·우클릭·재장전과 중복 실행되지 않는다.
회복은 적의 체력 또는 보호막을 실제로 줄인 발사에만 적용된다. 빗나감,
아군, 지형, 무적 대상은 회복을 발생시키지 않는다.

## 시간 영역

- 생성 시 바라보던 방향의 전방 1.5m부터 폭 8m, 길이 12m, 높이 4m.
- 10초 유지하며 이동하지 않는다. 바닥은 내부를 채우지 않은 노란 육각형 테두리들이다.
- 아군·적군·시전자 모두 영역 안에서 2초에 걸쳐 연속적으로 느려지고 멈춘다.
- 여러 영역에 겹쳐도 감속이 중첩되지 않는다. 영역을 벗어나거나 효과가 끝나면 풀린다.
- 진행 중인 공격·시전, 몸 애니메이션, 재장전도 함께 느려지며 시간 정지가 풀리면 이어진다.
- 벽으로 가려진 대상은 제외한다. 멈춘 상태에서는 이동·점프·포탈 관성으로 빠져나갈 수 없다.
- 사망·캐릭터 교체 시 시전과 소유 영역을 정리한다. 시전 전 기절·수면은 미발사 공격을 취소한다.

## 아트

- `assets/effects/renua-constructs-concept.png`: 내장 image_gen으로 그린 별도 시안.
- `assets/effects/renua-constructs-prompt.txt`: 사용한 전체 프롬프트.
- `src/presentation/renua_fx.rs`: 동심원, 눈금, 편심 궤도, 사각 문양, 마름모 장식을 독립적인 빈 리본 메시로 재구성.
- `assets/characters/skill/renua.json`: 원본 동작과 구조물 샘플.
- `renua.blend` / `renua.fbx`: 기존 메시와 얼굴, 기존 0–528프레임 키를 보존하고
  네 모션을 531–1337프레임에 추가. 망토 9본, 머리카락 4본을 추가했다.
- `artifacts/renua-skills/reference/source-contact.png`: 네 원본 모션의 단계별 검토 이미지.
- `artifacts/renua-skills/bake-verification.json`: 메시·기존 키 보존 및 추가 관절 검증 기록.

1인칭 양손, 외부 시점, 관전자와 사망 재생은 동일한 시전 상태·시간을 사용한다.
네트워크는 갱신된 공용 프로토콜을 사용한다. 기존 피격 표시 슬롯 안에 감속 정도를 함께 저장하고,
시간 영역 방향은 투사체의 남는 비트에 넣어 12명·96개 투사체 패킷 크기를 유지한다.

## 재생성과 확인

1. `node scripts/extract_renua_skills.cjs` — 보관된 참고 아틀라스 필요.
2. `python scripts/render_renua_reference.py` — Pillow / NumPy 필요.
3. Blender에서 `scripts/bake_renua_skills.py` 실행.
4. `cargo test --offline --no-default-features --test renua_skill --test combat`
5. `cargo test --offline --no-default-features --lib` — 로컬 UDP 허용 필요.
6. `cargo build --offline --bin tsb`
7. `target/debug/tsb.exe --solo --renua-skill-review`

리뷰 모드는 17개 외부/1인칭 장면을 `artifacts/preview-renua-skills-*`에 저장하고,
맨손 상태, 망토·머리카락 관절, 헤일로, 레이저와 시간 영역 표시를 실제 그래프에서 검사한다.

최종 리뉴아 전용 검사는 전투 10개와 네트워크 3개, GPU 장면 17개를 통과했다.
전체 회귀는 병행 중인 아야·힐데 변경 관련 실패가 있어 통과로 기록하지 않았다.
검사별 로그와 결과는 `artifacts/renua-skills/verification.json`에 모았다.
