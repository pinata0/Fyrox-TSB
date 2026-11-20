# 벨라 모션과 스킬

- 평상시 3인칭은 빈손이다. 1인칭은 오른손 손바닥을 완전히 펴고, 그 위에서 청백색·보라색 도깨비불이 회전하고 위아래로 떠다닌다.
- 저학년은 보관된 Spine `Skill1_1`의 100프레임(30fps)을 사용한다. 58프레임에 시전 방향으로 길이 10m, 폭 3m의 보라색 불꽃 구역을 만든다. 구역은 4초 지속하며 0.5초마다 기본 6 피해를 준다. 구역에 들어온 각 적에게 처음 적용에 성공했을 때 3초 도발을 건다. 피해 틱은 도발 시간을 연장하지 않는다. 벽 너머와 다른 층에는 적용하지 않는다.
- 도발은 부정적 상태이상이다. 이동·시점·기본공격·재장전은 유지하고 E, Q, 종족 이동기, 엘프 포탈 설치, 이드 우클릭 방출, 에르핀 케이크 먹기의 신규 사용을 막는다. 리뉴아 우클릭은 기본공격 분류를 유지한다. 이전의 도발자 외 대상 피해 감소는 제거했다. 이미 생성된 장판·투사체·강화 버프는 유지된다.
- 고학년은 `Ultimate1_1`의 150프레임(5초)을 사용한다. 64프레임에 실제 시야 내 적(조준 방향·수직 시야각·화면 비율·벽 가림 적용)에게 3초 도발을 적용한다. 면역 대상은 제외하며, 적용 성공 대상 비트마스크를 서버가 저장하고 복제한다. 비트 하나마다 불꽃 하나가 대상에서 랜턴으로 곡선을 그리며 들어온다.
- 고학년 시전 완료 시 현재 벨라 위치 반경 8m의 살아 있는 아군(자신 포함)에게 최대 HP의 40%를 한 번 회복하고 흰색 고리 파동을 발산한다. 회복량에는 기존 종족·치유량 보정이 적용되고 최대 HP를 넘지 않는다. 대상이 없어도 완료 시 회복한다. 죽음·기절·변신·도발 등으로 벨라 시전이 중단되면 종료 회복이 발생하지 않는다.
- 사거리, 폭, 피해, 지속시간, 회복 반경은 `src/bella_skill.rs`의 상수에 모았다. 명시되지 않은 수치는 초기 밸런스 값이다.

## 에셋

- 시안: `assets/effects/bella-lantern-concept.png`
- 편집 가능한 모델: `assets/effects/bella-lantern.blend`
- 교환 모델: `assets/effects/bella-lantern.glb`
- 실제 엔진에 로드하는 모델: `assets/effects/bella-lantern-mesh.json`
- 모션 샘플: `assets/characters/skill/bella.json`, 원본 SHA-256 포함
- FBX 구간: `BellaSkill` 531–631, `BellaUltimate` 634–784
- 원본 추출: `node scripts/extract_bella_skills.cjs`
- 베이크: Blender background로 `scripts/bake_bella_skills.py` 실행. 0–528프레임의 기존 키와 메쉬 정점 보존을 검사한다.
- 모델 재생성: Blender background로 `scripts/model_bella_lantern.py` 실행
- GPU 검수: `cargo run --bin tsb-review -- --bella-skill-review`
- 검수 결과는 `artifacts/preview-bella-skills-*.png`와 `artifacts/bella-skills/`에 저장된다.

스냅샷은 기존 4바이트 액션 상태 슬롯에 9비트 시간과 12비트 대상 마스크를 담는다. 장판 방향과 흰 파동 나이는 기존 투사체 패킷의 여유 비트를 사용한다. 패킷 크기를 늘리지 않는다.
