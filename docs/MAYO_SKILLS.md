# 마요 저학년·고학년

원본은 `assets/references/mayo-motion-reference.skel`의 Normal 스킨이다. `scripts/extract_mayo_skills.cjs`가 Spine 곡선과 IK를 30fps로 평가하여 `assets/characters/skill/mayo.json`을 만든다. 원본 총과 자세는 `artifacts/mayo-skills/reference/source-contact.png`에서 확인할 수 있다.

일반 공격은 무한 탄창이며 재장전하지 않는다. HUD에는 ∞로 표시한다. 일반 침·강화 독침·E/Q 독침은 1·3인칭의 실제 무기 총구에서 보이기 시작하고, 발사 시 총구 위치를 한 번 저장해 이후 손 움직임에 끌려가지 않는다. 조준과 명중 판정은 기존 서버 궤적을 유지한다.

## 저학년 E — 수집의 법칙임.

- `Skill1_1`, 2초. FBX 531–591프레임.
- 조준경·약병이 달린 소총을 꺼내 무릎을 꿇고 조준한다.
- 원본 24·28·32프레임에 독침을 한 발씩, 총 3발 발사한다.
- 각 발은 그 순간의 조준 방향으로 직진한다. 자동 추적과 좌우 부채꼴 분산은 없다.
- 기존 수치 유지: 발당 기본 피해 14, 속도 44m/s, E 쿨타임 9초. 일반 탄약을 소비하지 않는다.

## 고학년 Q — 그 수집품 내꺼임.

- `Ultimate1_1`, 100/30초. FBX 594–694프레임.
- 탄띠가 달린 4총열 총을 들어 올린 뒤 무릎을 꿇고 연사한다.
- 원본 49·53·57·61·65·69·73·77프레임에 총 8발 발사한다.
- 발사 방향·피해·속도·중독은 저학년과 같다. 고학년 적중 피해로 자신의 궁극기를 다시 충전하지 않는다.
- 총열 회전, 탄띠 소모, 반동과 녹색 총구 섬광은 같은 발사 시계를 사용한다.

## 중독

- 적중한 적은 4초간 주는 피해가 20% 감소한다. 지속 피해는 없다.
- 반복 적중은 지속시간을 갱신하며 감소율을 누적하지 않는다. 기존 일반 약화와는 같은 20% 감소 그룹을 사용한다.
- 기본·저학년·고학년 피해에 적용하며 이드의 고정 체력 비례 피해에도 반영한다. 기존 시온의 즉사 판정은 일반 피해 계산과 별도다.
- 무적, 스폰 보호, 디버프 면역, 정화를 존중한다. 기존 일반 공격의 강화 독침에도 같은 중독 정의를 적용한다.
- HUD와 F1 도움말에 피해 감소를 표시한다.

## 상태와 동기화

시전 중 일반 공격·재장전·다른 E/Q가 겹치지 않는다. 이동과 조준은 가능하다. 사망·캐릭터 변경·기절·수면·동결·변이·잠복은 남은 연사를 취소한다. 시간 감속/정지는 시전 시계를 늦추거나 멈춘다.

서버만 투사체와 피해를 생성한다. 기존 캐릭터별 액션 비트와 시계 필드를 재사용하므로 스냅샷 크기는 늘어나지 않는다. 중독 활성 여부는 기존 상태 비트로 전송하며 지속시간은 서버가 관리한다. 클라이언트 예측·원격 관찰·리플레이는 같은 모션 시계를 사용한다. 호스트와 클라이언트를 함께 업데이트해야 한다.

## 제작물과 재생성

- 콘셉트: `assets/effects/mayo-guns-concept.png` — 내장 imagegen으로 생성. 최종 프롬프트는 `mayo-guns-prompt.txt`.
- 편집 원본: `assets/effects/mayo-guns.blend`.
- 독립 모델: `mayo-skill-gun.glb`, `mayo-ultimate-gun.glb`.
- 게임 메시: `mayo-guns.json`. 두 무기 합계 10,080삼각형. 총열과 탄띠는 독립 그룹이다.
- `scripts/model_mayo_guns.py`로 모델을 재생성한다.
- `scripts/bake_mayo_skills.py`는 기존 0–528프레임 키와 모든 메시 좌표를 보존했는지 검증하고, 최초 백업을 `artifacts/mayo-skills/before`에 저장한다. 재실행 시 현재 파일의 다른 수정도 보존한다.

```powershell
node scripts/extract_mayo_skills.cjs
& 'C:/Program Files/Blender Foundation/Blender 5.3/blender.exe' --background --python-exit-code 1 --python scripts/model_mayo_guns.py
& 'C:/Program Files/Blender Foundation/Blender 5.3/blender.exe' --background --python-exit-code 1 --python scripts/bake_mayo_skills.py
cargo test --no-default-features --test mayo_skill
cargo test --no-default-features --lib mayo
cargo build --bin tsb-review
.\Play.cmd --mayo-skill-review
```

모션 추출 시 공개 원본 아틀라스 `https://cheeky.tsuncloud.com/deploy/com.epidgames.trickcalrevive/spines/spine/ingame/hero/mayo/mayo/skeleton.atlas`와 같은 폴더의 `Mayo.png`가 `artifacts/mayo-skills/reference`에 필요하다.

## 검증

- 무한 탄창·총구 수정: 마요 전투 9개, 공통 전투 35개, 마요·네트워크 단위 3개, 총구 보정 7개 통과. R을 누르며 40발 연속 발사, 강화 독침 주기, 0발 상태의 네트워크 왕복, 총구에서 출발 후 움직이는 무기로부터 분리되는 궤적을 확인했다.
- 수정 후 1·3인칭 GPU 검증 13장 통과. 일반 발사관·E 소총·Q 회전 총열의 총구와 침 끝의 오차 2mm 미만, 손 접점과 기본 무기 복귀 확인. `artifacts/mayo-skills/muzzle-review.log`.
- 전용 전투 테스트 8개, 중독·네트워크 단위 테스트 3개 통과. 원본 시점의 3/8발, 조준 변경, 중단, 시간 정지, 중독 갱신, 모든 일반 피해 종류의 감소, 패킷 크기 유지 확인.
- 1·3인칭 GPU 검증 12장 통과. 두 손의 총 접점, 준비/발사 자세, 기본 무기로 복귀 확인. `artifacts/mayo-skills/gpu-sleeve.log` 및 `final-contact.png`.
- 에셋 검사: 162프레임, 기존 키·메시 보존, 원본 해시, 총 메시 그룹·인덱스 정상. `asset-verification.json`.
- `cargo build --offline --bin tsb-review` 기본 기능(EOS 포함) 빌드 성공.
- 전체 전투 회귀 검사는 동시 작업 중인 에르핀·피코라 관련 6개가 실패하여 전체 통과로 간주하지 않는다. 마요 전용 검사는 모두 통과했다.
- 일반 `--smoke-test`는 모든 15명 에셋을 로드했으나 14초 내 렌더·궁극기·댄스 시퀀스를 완료하지 못해 `SMOKE_FAILED`로 종료했다. 정상 실행 성공으로 기록하지 않는다. 로그는 `artifacts/mayo-skills/smoke-final.log`와 `.err`.
- 기존 본체의 소매 경계에 남아 있는 거친 표면은 기존 기본 공격 화면에도 나타나며 이번 모션 추가에서 본체 메시를 변경하지 않았다.
