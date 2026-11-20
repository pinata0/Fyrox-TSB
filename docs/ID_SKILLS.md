# 이드 모션과 전투 효과

2026-11-19 구현. 보관된 `assets/references/id-motion-reference.skel`의
`Skill1_1`과 `Ultimate1_1`을 30fps로 읽어
`assets/characters/skill/id.json`에 기록했다. 현재 FPS 규칙은 아래와 같다.

| 입력 | 전투 효과 | 모션·화면 |
| --- | --- | --- |
| 평상시 | — | 두 눈을 감고 탈것에 기대어 있음. 머리카락 3개 체인과 느린 황색 궤도 |
| 좌클릭 | 나타의 단발 레이저, 기본 피해 28, 사거리 25m, 간격 0.8초, 기존 탄창 40 유지 | 3인칭 이드는 정지하고 나타에서 빛나는 레이저 발사. 1인칭은 손등을 보인 펼친 손에서 잠깐 검지 지시, 화면 바깥에서 조준점으로 레이저 |
| E 저학년 | 재사용 16초. 자신 3초 무적, 살아 있는 모든 아군은 같은 3초 동안 체력 10% 아래로 내려가지 않음 | 원본 3.6초. 탈것 패널·머리카락 동작. 1인칭 카메라 최대 6.5cm 상승, 노란 사선 프레임·경고·데이터·글리치 UI가 0.78초 이내 퇴장 |
| 우클릭 | 재사용 3초, 반경 6m, 기본 피해 15와 작은 밀치기. 한 번 누를 때 한 번 발동 | 0.7초 동안 옅은 흰 고리 확산, 탈것 양쪽 랜턴 발광. 1인칭 흰 UI 0.5초 |
| Q 고학년 | 원본 5.333초 동작 중 받는 피해 30% 감소. 2.133초 차징 후 반경 10m 적에게 최대 체력 50%의 피해를 한 번 적용 | 탑승부에 검은 원형 덮개, 탈것 부양, 개방과 동시에 양쪽 눈을 뜨고 서로 기울어진 두 궤도 전개. 종료 전에 다시 눈을 감음 |

우클릭 범위·기본 피해·밀치기와 Q 범위는 이번 구현의 초기 밸런스값이다.
공격은 서버가 판정하며 범위 공격은 지형의 차폐를 따른다. Q의 50%는 공격자 피해
증폭으로 커지지 않으며, 피해자의 보호막·방어·무적·체력 하한은 정상 적용된다.
저학년의 아군 보호는 횟수 제한이 없다. 이미 10%보다 낮은 생존 아군은 10%로
맞추며 사망자는 부활시키지 않는다. 기절·수면·땅굴 진입은 진행 중 모션을 취소하고,
Q가 발사 전에 취소되면 피해도 발생하지 않는다. 이미 부여한 E 보호는 남은 시간 유지된다.

## 편집 가능한 모델

`assets/characters/id.blend`와 `id.fbx`에 다음을 반영했다.

- 탈것: `IdPodMesh.Seat`, `Bow`, `Back`, `Crown`, `Lantern.L`, `Lantern.R`의 6개 실제 메시와 대응하는 `IdPod.*` 본.
- 머리카락: `IdHair.L/C/R.1/2/3`, 총 9개 추가 본. 기존 머리카락 가중치만 분산.
- 전체 69개 본. 기존 캐릭터 몸체 좌표·토폴로지·23개 얼굴 변형은 보존.
- 탈것의 11,998개 면은 중복·누락 없이 분리하며 UV와 머티리얼을 보존.
- 새 클립: `IdSkill` 531–639, `IdPulse` 642–663, `IdUltimate` 666–826 (30fps).
- 일반 공격 156–180은 몸체·머리·팔을 고정하고 머리카락만 2차 움직임을 유지.

눈과 덮개·궤도·랜턴의 가시성은 서버 동작 시간에 맞추어 렌더링한다.
발광 셰이더는 툰 렌더러의 블룸 설정과 무관하게 밝은 중심과 주변 광을 표시한다.
1인칭 UI는 메뉴·대화·캐릭터 선택·궤도 카메라에서 표시하지 않는다.
보호막 구체는 3인칭에만 표시하여 1인칭 화면에는 지속적인 격자가 남지 않는다.
스킬 단계·진행 시간·무적·우클릭 쿨다운은 기존 패킷 슬롯을 사용하여 원격 관전자와
늦게 접속한 플레이어에게도 전송한다.

## 재생성 및 확인

```powershell
node scripts/extract_id_skills.cjs
$blender = 'C:/Program Files/Blender Foundation/Blender 5.3/blender.exe'
& $blender -b --python-exit-code 1 --python scripts/bake_id_skills.py
& $blender -b --python-exit-code 1 --python scripts/export_meshy_morphs.py -- assets/characters id
& $blender -b --python-exit-code 1 --python scripts/verify_id_skills.py
cargo test --no-default-features
cargo run --no-default-features --features desktop --bin tsb -- --id-skill-review
```

베이크는 최초 실행 시 `artifacts/id-skills/before`에 기록한 원본에서 다시 만든다.
이 원본은 덮어쓰지 않는다. 검증기는 원본과 현재 몸체·표정·탈것 UV/면을 비교하고,
공격 자세의 정지, 각 패널과 머리카락 체인의 독립 움직임을 검사한다.
검증 결과는 `artifacts/id-skills/asset-verification.json`에 남는다.
GPU 검사는 1·3인칭 16개 장면을 `artifacts/preview-id-skills-*.png`에 저장한다.

주요 구현은 `src/id_skill.rs`, `src/presentation/id_fx.rs`,
`src/presentation/id_hud.rs`, `src/presentation/id_skill_review.rs`에 있다.
