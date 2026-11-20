# 3인칭 무기 크기 조정

15명의 저장된 원작 전신 자료(`assets/references/*-fullbody.png`)와 실제 게임의 전신·무기 화면을 비교했다. 아야는 꽃잎 우산 자료(`assets/references/aya-parasol-reference.webp`)와 제작용 일러스트도 함께 확인했다.

모든 월드 무기에 적용하던 0.70배 축소 때문에 긴 지팡이와 총, 아야의 우산이 캐릭터에 비해 작아졌다. 원작의 몸·머리 대비 실루엣을 기준으로 다음 7명을 조정했다. 수치는 2D 원작을 현재 3D 모델에 맞춘 시각적 조정이며 공식 실측 치수가 아니다.

| 캐릭터 | 무기 | 이전 대비 | 월드 배율 | 비교 기준 |
|---|---|---:|---:|---|
| 아야 | 꽃잎 우산 | 2.00배 | 1.40 | 머리와 어깨를 덮는 큰 꽃잎 캐노피 |
| 에르핀 | 왕실 지팡이 | 2.21배 | 1.55 | 발 근처에서 왕관 위까지 이어지는 길이 |
| 피코라 | 장식 지팡이 | 2.86배 | 2.00 | 몸 높이에 가까운 긴 손잡이와 큰 끝장식 |
| 우이 | 클로버 지팡이 | 3.14배 | 2.20 | 모자 위로 올라오는 클로버 |
| 버터 | 새총 | 1.43배 | 1.00 | 얼굴 크기에 가까운 새총 가지 |
| 힐데 | 의료 총 | 2.36배 | 1.65 | 몸통을 가로지르는 긴 총 실루엣 |
| 시온 | 장총 | 3.14배 | 2.20 | 몸 높이보다 긴 대형 총 |

비비의 부유검과 이드·클로에의 몸에 붙은 탑승물/동료는 이번 조정 대상에서 제외했다. 나이아·리뉴아·마요·벨라·코미는 저장된 대기 자료만으로 확대 근거가 명확하지 않아 기존 크기를 유지했다.

## 적용

`src/presentation/weapons.rs`의 `world_scale()`에서 캐릭터별 월드 배율을 관리한다. `characters.rs`는 손바닥에 무기를 배치할 때 실제 스케일로 손잡이 오프셋을 계산한다. 양손 무기의 보조 손도 확대된 무기 좌표를 사용해 맞춘다. 1인칭 모델과 전투 수치는 유지된다.

## 검증 재현

### 머리 관통 보정 (2026-11-21)

15명의 3인칭 대기·공격을 정면과 측면에서 확인하고, 피코라의 장식 지팡이와 우이의 클로버가 머리·모자에 파묻히는 부분을 보정했다. `weapons::world_pitch()`가 피코라는 30도, 우이는 35도 앞으로 기울인다. 위쪽 조준이나 공격의 뒤로 젖히는 동작에서도 이 최소 앞기울기를 유지한다. 손바닥을 중심으로 회전하므로 손잡이 파지, 양손 IK와 무기 끝 이펙트가 함께 따라간다. 1인칭 각도와 무기 크기는 기존 값을 사용한다.

수정 후 전원 정면·측면 대기/공격 60장(파지 오차 1mm 미만), 우이 스킬 15장, 피코라 스킬 13장의 GPU 검사가 통과했다. 개발용·release 빌드가 성공했으며 `Play.cmd`로 실행한다. Clippy의 경고 오류화 검사는 이번 수정 밖의 `combat.rs`, `game.rs`, `butter_skill.rs`에 있는 기존 경고 4건 때문에 통과하지 못했다. [전후 비교](../artifacts/weapon-head-clearance/comparison.png)와 검증 로그·캡처는 `artifacts/weapon-head-clearance/`에 보관한다.

```powershell
cargo run --bin tsb -- --solo --auto-start --weapon-review --world-weapons-only --hud-review-small
cargo run --bin tsb -- --solo --auto-start --weapon-review --world-weapons-only --weapon-side-view --hud-review-small
cargo run --bin tsb -- --solo --auto-start --weapon-review --hud-review-small
```

앞의 두 실행은 각각 15명의 대기·공격 30장을 촬영하고 손바닥 접촉 오차 1mm 미만을 검사한다. 정면에서 짧게 보이는 총의 길이는 측면에서도 비교한다. 마지막 실행은 1인칭·재장전·조준·인사·복귀까지 포함하는 전체 무기 회귀 검사다.

새 등장 모션이 긴 캐릭터에서 무기가 숨겨진 채 대기 검사를 하던 기존 검증 문제도 수정했다. 월드 대기 검사는 동일한 Idle 시점을 고정한다.

수정 전후 비교는 현재 에셋과 동일한 검증 코드를 쓰되, 격리된 비교용 소스에서만 월드 배율을 이전의 0.70으로 고정했다. 해당 소스·실행 로그·원본 캡처·크기 목록은 `artifacts/world-weapon-size/`에 보관한다.

최종 빌드는 같은 파일에서 별도로 진행된 힐데의 총 방향·손잡이 정렬 수정도 보존한다. 따라서 힐데 비교 화면에는 크기 외에 이 방향 교정도 나타난다.

## 최종 결과

- `cargo fmt --all -- --check`, `cargo clippy --locked --offline --all-targets -- -D warnings` 통과.
- 단위·통합 테스트 222개 통과. 로컬 UDP 테스트는 샌드박스 밖에서 실행했다.
- 최종 빌드의 전체 무기 검사 102장과 측면 대기·공격 30장 통과. `WEAPON_REVIEW_OK`, `RIGHT_HAND_ROSTER_OK`, `WORLD_WEAPONS_OK` 확인.
- 단일 EXE의 `--verify-only` 종료 코드 0 확인. 실제 추출된 1,333개 배포 파일의 해시를 대조했고, 생성된 안내문을 더해 총 1,334개 파일이 포함된다.
- 배포 파일: `release/TSB-Multiplayer-WorldWeaponSize.exe`.
- EXE SHA-256: `a10e5209cef2df68e44819756afa71fee1ccde8ab35b5da39a92cbad31ac67de`.

[아야 전후 비교](../artifacts/world-weapon-size/aya-before-after.jpg), [아야·에르핀·피코라·우이 비교](../artifacts/world-weapon-size/comparison-1.jpg), [버터·힐데·시온 비교](../artifacts/world-weapon-size/comparison-2.jpg). 상세 검증 결과는 `artifacts/world-weapon-size/validation-report.json`에 있다.
