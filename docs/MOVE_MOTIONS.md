# 캐릭터별 Move와 이동 중 팔 공격 / 2026-11-16

보관된 15명 전투 Spine의 `Move` 트랙을 곡선 보간·IK까지 평가해 30Hz로 추출했다. 게임의 Walk(63–93), Run(96–120)을 각 캐릭터의 Move로 교체했다. 걷기는 원본 주기, 달리기는 같은 동작을 1.25배 속도로 재생한다. 원본 좌우 손은 기존 무기·평타 매핑을 따르며 버터의 반대쪽 새총 손도 유지한다.

비비·리뉴아·벨라의 부유 자세, 이드의 앉은 자세, 우이의 양발 도약, 나머지 캐릭터의 발 교대·팔 동작을 각각의 표본에서 가져온다. 현재 모델의 Idle을 기준으로 2D 관절 회전을 3D 전후 방향에 옮기며, 관절 한계·상하 진폭·바닥 관통 보정·머리카락과 망토의 후속 움직임은 3D용 각색이다. 이동 위치와 충돌은 기존 게임 로직이 담당한다.

| 캐릭터 | Move 주기 |
|---|---:|
| 비비 | 1.333초 |
| 나이아 | 1.333초 |
| 에르핀 | 1.467초 |
| 이드 | 2.667초 |
| 아야 | 0.800초 |
| 피코라 | 0.933초 |
| 클로에 | 0.800초 |
| 리뉴아 | 1.000초 |
| 마요 | 0.800초 |
| 벨라 | 6.600초 |
| 우이 | 0.800초 |
| 버터 | 0.800초 |
| 코미 | 0.933초 |
| 힐데 | 0.800초 |
| 시온 | 0.667초 |

## 이동 중 공격

`Pose.locomotion`이 보행 시간을 별도로 보존한다. 공격 재시작·연사·공격 가속은 보행 시간을 초기화하지 않으며, Walk↔Run 전환도 같은 발 주기를 이어간다. 전신에는 Move를 재생하고 `UpperArm.L/R` 아래의 팔·손·손가락에만 기존 Attack/Attack2를 블렌딩한다. 골반·몸통·머리·다리·보조물은 Move를 유지한다. 시작과 끝에는 짧게 블렌딩하고 이후 무기 파지 IK를 적용한다.

정지 중 공격은 기존 전신 평타를 유지한다. 점프·스킬·궁극기·사망·선택 변경에서는 이동 레이어를 해제한다. 비비의 독립 수은검과 아야의 강화 양산 변형, 1인칭 손, 나이아의 공격 표정은 기존 공격 시각을 사용한다. 데스캠은 두 시각을 함께 기록하고 보간한다. 전투 수치·네트워크 패킷 변경은 없다.

## 재생성·검증

```powershell
node scripts/extract_move_motions.cjs
& 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe' --background --python-exit-code 1 --python scripts/bake_move_motions.py
cargo test --offline --no-default-features --lib motion::
cargo test --offline --no-default-features --lib deathcam::
cargo run --bin tsb -- --solo --attack-review --move-review
```

- 원본 표본·SHA-256·본 매핑: `assets/characters/move/motions.json`. 원본은 `assets/references/manifest.json`에 출처가 기록된 보관본이다.
- 15개 Blend/FBX와 manifest를 갱신했다. 30개 반복 구간의 연결부·동작 변화, 다른 애니메이션 키, 모든 스킨 메시·가중치·모프 보존 검사를 통과했다. 상세: `artifacts/move-motions/asset-verification.json`.
- 원본 5시점 관절 비교: `artifacts/move-motions/research/source-poses.png`.
- 모션 테스트 14개, 데스캠 테스트 12개 통과. 데스크톱 빌드와 `cargo clippy --bin tsb --lib -- -D warnings` 통과.
- GPU 검증은 15명 × 걷기/달리기/걷기 공격/달리기 공격 × 4시점, 총 240장의 JPEG 캡처를 완료하고 종료 코드 0으로 통과했다. 골반·몸통·머리·다리·탑승물의 변환 7,200개가 Move 표본과 일치했다. 비비·아야의 달리기 공격에서는 Attack2도 검증했다. 결과는 `artifacts/move-motions/runtime-verification.json`, 전체 로그는 `gpu-final.log`, 전원 비교는 `contact-0.png`, `contact-5.png`, `contact-10.png`에 있다.
- 전체 라이브러리 테스트의 네트워크/포털 실패는 `artifacts/move-motions/rules-tests.log`, 전체 타깃 Clippy의 별도 포털 테스트 경고는 `clippy.log`에 기록했다. 이 항목들을 모션 검증 통과에 포함하지 않는다.
- 변경한 Rust 파일의 포맷 검사는 통과했다. 전체 포맷 검사에서 보고된 동시 작업 중의 포털 코드 차이는 `fmt.log`에 남겼다.

변경 전 코드·모델은 `artifacts/move-motions/before/`에 보존한다. 런타임은 FBX를 사용하며 Spine 원본·분석 런타임은 사용하지 않는다.

## 실행 파일

작업 폴더의 `Play-MoveMotions.cmd`는 올바른 에셋 경로에서 빌드된 `target/debug/tsb.exe`를 실행한다. 단일 EXE 포장은 C: 드라이브 공간 부족으로 완료하지 않았다. 이전 `release/` 배포본에는 이 변경이 포함되어 있지 않다. 처음 실행한 릴리스 링크 실패는 `release-build.log`에 기록했다. 임시 테스트 캐시를 정리하고 캡처를 JPEG로 전환했으며, 이후 릴리스 재빌드는 남은 디스크 공간을 보호하기 위해 중단했다.
