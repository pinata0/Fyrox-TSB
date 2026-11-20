# 원작 일반 평타 모션

## 피코라 모션·스티커 / 2026-11-20

피코라는 아래와 동일한 보관 Spine 원본을 사용한다. `Attack1_1`의 47프레임 모션을 기존 0.470초로 재생하며, 18프레임 이벤트(게임 시계 0.180초)에 파란 마력탄을 발사한다. E는 `Skill1_1`, Q는 `Ultimate1_1`을 추가했다. Q 점프는 전신에만 적용하고 1인칭은 지팡이 섬광만 표시한다. E/Q 효과와 검증은 [피코라 스킬](PICORA_SKILLS.md)을 참조한다. 아래 과거 기록의 즉시 발사·네 번째 튕김·E/Q 제외 범위는 피코라에 대해 이 변경으로 대체한다.

## 나이아·힐데·시온 1인칭 반동 / 2026-11-19

세 캐릭터의 1인칭 평타는 발사 직후 총구를 각각 약 5.7°·6.9°·8.6° 위로 들고, 손잡이도 1.8–2.4cm 들어 올린 뒤 부드럽게 원위치로 돌아온다. 공격 진행률 16%에서 최대 반동, 86%에서 복귀하며 연사·공격 속도 효과·데스캠은 공통 공격 시계를 따른다. 양손은 기존 파지점을 따라간다. 전신 원작 모션과 나이아의 스킬·궁극기 동작은 유지한다.

구현은 `src/attack.rs::first_person_hand`와 `src/presentation.rs`에 있다. `--solo --auto-start --attack-review --recoil-only --hud-review-small` 실행 시 세 캐릭터의 대기·상승·최대 반동·복귀 18개 화면을 캡처하고 손잡이 접촉을 검사한다.

## 아야 양산 변형 / 2026-11-15

아야에 `Attack1_1 → Attack1_1 → Attack2_1`의 전신·1인칭 동작과 양산 접힘/검·언월도 변형을 적용했다. 기존 세 번째 강화 피해에 모션을 연결하며 전투 수치는 유지한다. 아래 과거 기록의 강화 모션 공유·무기 변신 제외 범위는 아야에 대해 이 변경으로 대체한다. [세부 구현 및 검증](AYA_WEAPON_TRANSFORM.md).

## 비비 네 칼날 좌우 베기 / 2026-11-20

비비는 `Attack1_1`의 좌→우 0.8초 / 우→좌 1.1초를 나누어 사용한다. 탭은 좌→우 한 번, 홀드는 좌우 반복, 동작 종료 후 다시 누르면 좌→우로 초기화한다. 손은 빈손 제스처이며 등 뒤 칼날 네 개가 독립적으로 움직인다. 두 베기 모두 기본 26 피해이고 원작 이벤트 시점에 한 번 판정한다. 이전 3단 콤보와 `Attack2_1` 마무리는 사용하지 않는다. 대기 날개·E·Q, 창 모델, 재생성 및 검증은 [비비 모션 문서](VIVI_SKILLS.md)를 참조한다.

2026-11-14. 현재 로스터 15명의 기본 전투형 `Attack1_1`을 조사하고 기존 3D 모델과 1인칭 손·무기에 적용했다.

## 조사 근거

`assets/references/manifest.json`에 URL·제작자(EPID Games)·자료 호스트(Cheeky)가 기록된 전투 Spine 원본을 사용했다. 기존 보관 원본을 Spine 런타임으로 평가하므로 단순 키 값뿐 아니라 곡선 보간과 IK가 반영된다. 원본 파일의 SHA-256, 본 매핑, 다른 Attack 트랙 목록, 30Hz 관절 좌표와 Event 마커는 `artifacts/basic-attacks/research/motion-report.json`에 보관한다. 이번 작업에서 뷰어 페이지의 웹 본문은 조회되지 않아 직접 보관된 원본 트랙을 분석했다.

일반 공격인 `Attack1_1`을 강화 공격 `Attack2_*`, 변신형 및 의상 전용 트랙과 구분했다. 강화 공격의 전투 효과는 기존 구현을 따르며 이번에는 일반 평타 모션을 공유한다. 아래 재생 시간은 기존 FPS 공격 간격의 94%이며, 원작의 전체 공격 주기와 같다고 주장하지 않는다.

| 사도 / 원작 뷰어 | 원작 일반 평타 | FPS 재생 길이 |
|---|---:|---:|
| [비비](https://wiki.cheeky.tsuncloud.com/korea/spines/vivi) | 1.900초 | 0.658초 |
| [나이아](https://wiki.cheeky.tsuncloud.com/korea/spines/naia) | 1.667초 | 0.357초 |
| [에르핀](https://wiki.cheeky.tsuncloud.com/korea/spines/erpin) | 1.467초 | 0.517초 |
| [이드](https://wiki.cheeky.tsuncloud.com/korea/spines/ed) | 2.000초 | 0.752초 |
| [아야](https://wiki.cheeky.tsuncloud.com/korea/spines/aya) | 1.500초 | 0.658초 |
| [피코라](https://wiki.cheeky.tsuncloud.com/korea/spines/picora) | 1.567초 | 0.470초 |
| [클로에](https://wiki.cheeky.tsuncloud.com/korea/spines/chloe) | 2.033초 | 0.752초 |
| [리뉴아](https://wiki.cheeky.tsuncloud.com/korea/spines/renewaawaken) | 2.700초 | 0.376초 |
| [마요](https://wiki.cheeky.tsuncloud.com/korea/spines/mayo) | 1.467초 | 0.329초 |
| [벨라](https://wiki.cheeky.tsuncloud.com/korea/spines/vela) | 2.600초 | 0.658초 |
| [우이](https://wiki.cheeky.tsuncloud.com/korea/spines/ui) | 1.500초 | 0.423초 |
| [버터](https://wiki.cheeky.tsuncloud.com/korea/spines/butter) | 1.500초 | 0.611초 |
| [코미](https://wiki.cheeky.tsuncloud.com/korea/spines/kommy) | 1.400초 | 0.752초 |
| [힐데](https://wiki.cheeky.tsuncloud.com/korea/spines/hilde) | 1.933초 | 0.376초 |
| [시온 더 다크불릿](https://wiki.cheeky.tsuncloud.com/korea/spines/xxionx) | 2.300초 | 0.799초 |

## 적용 범위와 각색

- 원작 15관절의 상대 회전, 몸 이동, 양손의 궤적을 각각 추출한다. 비비·아야의 연속 동작, 사격 준비·반동·복귀를 하나의 공통 사인 곡선으로 대체하지 않는다.
- 원작의 무기 연결 본을 기준으로 좌우를 매핑한다. 대부분 `Hand_L`이 무기 손이며, 버터의 새총 몸체는 `Hand_R`을 따른다. 3D 모델에서는 실제 오른손인 `Hand.L`을 유지한다.
- Blender/FBX의 Attack 구간(156–180프레임)만 교체한다. 원래 메시·가중치·재질과 나머지 애니메이션 키는 보존 검사를 한다. 팔의 회전과 원작 2D 움직임을 전진/상하 방향으로 옮기며, 깊이·관절 한계·바닥 접촉·짧은 대기 자세 블렌드는 3D에 맞춘 각색이다.
- 1인칭은 같은 원작 양손 궤적을 사용하되 시야를 가리지 않도록 진폭을 줄인다. 비비는 한 번의 평타에서 두 검이 독립적으로 움직이고, 버터는 왼손으로 새총 줄을 당기며 고무줄·돌도 따라 움직인다. 기존 캐릭터의 실제 팔과 손잡이 접촉을 유지한다. 카메라 원점 대신 손잡이 중심으로 회전해 준비 동작 중 손이 카메라 뒤로 넘어가지 않게 한다.
- 전신·1인칭·자기 그림자·녹화 데스캠은 같은 공격 진행 시각을 사용한다. 연사는 새 공격에서 다시 시작하며 공격 속도 버프·거대화·감속을 반영한다. 점프 도중에도 시작한 평타를 끝까지 보여주고 사망·캐릭터 교체는 초기화한다.
- 피해량, 판정 시점, 탄약, 공격 간격, E/Q 효과 및 네트워크 프로토콜은 이번 변경의 대상이 아니다. 판정은 기존 서버 즉시 발동 방식을 유지하므로 원작 Event 마커와 실제 피해 판정 시간을 일치시킨 구현은 아니다.
- 2D 파츠 교체, 스프라이트 변형, 원작 전용 무기 변신과 이펙트를 픽셀 단위로 복제하지 않는다. 나이아는 후속 작업에서 원작 눈·입 전환 시점과 좌우 땋은 머리 체인을 추가했다([나이아 평타·표정](NAIA_ATTACK.md)). 완성된 3D 리타기팅이며 원작 애니메이션의 그대로 재생이 아니다.

## 재생성

```powershell
node scripts/extract_attack_motions.cjs
& 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe' --background --python-exit-code 1 --python scripts/bake_attack_motions.py
cargo run --bin tsb -- --solo --attack-review
```

다른 모델 제작 스크립트로 캐릭터를 다시 생성한 경우 이 베이크를 후처리로 실행한다. 런타임은 원본 Spine/조사 런타임을 로드하지 않으며, FBX와 양손 수치 표본만 사용한다. 변경 전 코드와 모델은 `artifacts/basic-attacks/before/`에 있다.

## 검증

`--attack-review`는 전원 여섯 시점 × 전신/1인칭, 총 180장 GPU 캡처를 저장한다. 전신 관절을 Blender 체크포인트와 비교하고, 월드/1인칭 손잡이 접촉 오차가 1mm 미만인지 확인한다. 캡처는 `artifacts/preview-attack-*`, 로그·베이크 보존 검사·자동 테스트 결과는 `artifacts/basic-attacks/`에 저장한다.

최종 확인: 자동 규칙/UDP 테스트 179개 통과, `cargo fmt --check`와 전체 타깃 Clippy 경고 0개. Blender에서 15개 리그·165개 클립·60개 루프 검사 통과. 평타 GPU 180개 장면과 360개 전신 관절 체크포인트 검사 통과, 모든 검사 대상 손잡이 접촉 오차 1mm 미만.

추가 무기 회귀 검사 102개(재장전·인사·좌우 손·외부 시점) 통과. `release/TSB-Multiplayer-AttackMotions.exe` 단일 EXE의 해시/캐시 검증과 실제 그래픽 실행도 종료 코드 0으로 완료했다. 실행 파일 SHA-256: `84283183441f480c9284c565509f952fb999205bcb1f38ddc7d9d1ca5275e63f`.

검증 시점 주의: 위 자동 테스트/Clippy 통과 이후 공유 작업 폴더에 라운드 전환 UI가 병행 추가되었다. 마지막 전체 소스 재검사에서는 `src/presentation/round_results.rs`의 `text(..., title.into())` 형식 추론 오류(E0283)가 보고되었다(`artifacts/basic-attacks/clippy-complete.log`). 해당 병행 변경은 이 평타 작업의 수정 범위가 아니며, 제공 EXE는 이미 성공한 릴리스 빌드와 패키지 실행 검증 결과다.
