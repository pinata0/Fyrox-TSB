# 팔과 머리카락·의상 리깅 분리

15명 공통 자동 리깅의 팔 가중치가 긴 머리, 망토, 몸통 의상에도 남아 있었다. 손과 소매를 별도 메시로 교체하는 후처리는 원래 팔의 면만 제거했기 때문에, 본체의 잘못된 가중치는 공격·인사·춤에서 계속 작동했다.

## 수정

- 별도로 제작한 `RefinedHand`, `RefinedSleeve`, `RefinedViewSleeve`는 기존 스킨 가중치를 유지한다.
- 남은 `SkinnedMesh` 본체에서는 위팔과 그 자식 관절의 영향을 모두 제거한다. 기존 머리·머리카락·몸통·골반 가중치를 유지하고 정규화하며, 팔 가중치만 남은 영역은 연결된 표면을 따라 신뢰할 수 있는 비팔 가중치를 복원한다. 서로 가까운 머리카락과 몸통을 공간 거리만으로 연결하지 않는다.
- 리뉴아의 복원 외투는 실제 소매 주변에만 위팔·아래팔 영향을 남긴다. 몸통과 늘어진 패널은 몸통·골반으로 전환하며, 손목과 손가락 회전이 소매를 비틀지 않게 한다.
- 정점 위치, 면, UV, 23개 얼굴 모프, 관절 위치, 기존 모션 키는 그대로 유지한다. 나이아의 독립 땋은 머리 관절도 유지한다.
- `refine_character_hands.py`와 `restore_renua_coat.py`에서 같은 보정을 실행해 재생성 시 재발을 방지한다. 아직 별도 팔 메시가 없는 원본 모델에 무조건 적용하지 않는다.

## 재현 및 검증

Blender 백그라운드에서 다음 순서로 실행한다.

1. `scripts/fix_arm_weight_isolation.py`: 현재 15명 원본을 `artifacts/arm-isolation/before/`에 보존하고 `characters/`에 수정본 생성.
2. `scripts/verify_arm_weight_isolation.py`: 원본과 메시·UV·얼굴 모프·관절·모션 키를 비교하고, 실제 스킨 변형과 FBX 재가져오기 검사.
3. `scripts/publish_arm_weight_isolation.py`: 전체 검사 통과 및 원본/수정본 해시 일치 후 게임 에셋에 적용. 최신 manifest의 다른 필드는 보존.
4. `scripts/export_meshy_morphs.py -- assets/characters`: 최종 FBX의 얼굴 모프 연결 데이터 생성.
5. `TSB_CHARACTER_ASSET_DIR`을 `assets/characters`의 절대 경로로 지정해 검증 스크립트를 다시 실행.

팔만 크게 회전하는 격리 검사에서 본체의 최대 변위는 0이어야 한다. 실제 손은 10cm 이상 움직여야 하며, 리뉴아의 팔 영향이 없는 외투 패널도 움직이지 않아야 한다. 모든 모션을 5개 시점에서 검사하고, 6mm보다 긴 메시 모서리의 늘어남 99백분위가 기존 에셋 검증 기준인 3.2배 미만인지 확인한다.

결과는 `artifacts/arm-isolation/verification.json`, 수정 전후 렌더와 실행 검증 로그도 같은 폴더에 보관한다. 저장된 원본 백업을 사용하므로 후속 모델 수정 이후 재실행하려면 새 작업 폴더에서 새 원본을 보존해야 한다.

15명 총 166개 모션의 830개 시점이 통과했다. 팔 격리 검사에서 본체 최대 변위는 캐릭터별 0.525~1.555m에서 모두 0m로 줄었다. 모서리 늘어남 99백분위의 전체 최댓값은 1.932배였다. 검증에서 정점·면·UV·모프와 관절·모션 키를 원본과 대조하고, FBX 15개를 독립적으로 다시 가져왔다.

검증 도중 별도 작업에서 추가된 나이아 스킬(최종 프레임 672)은 작업 폴더에 보존하고 가중치 수정만 병합했다. 해당 병합 전 모델은 `concurrent-before/naia.blend`이며, `TSB_ARM_BASELINE_DIR`로 이 폴더를 지정하면 나이아는 이 원본과, 나머지는 최초 원본과 비교한다. `bake_naia_skill.py`에도 동일한 가중치 보정을 연결했다. 단독 테스트 배포 폴더 `artifacts/releases/TSB-ArmRigFix-20261115`에는 기존 실행 파일과 호환되는 검증된 나이아 모션 범위를 사용한다.

게임 실행 검사도 종료 코드 0으로 통과했다. `--art-review`로 15명 앞·뒤 춤 화면 30장을, `--weapon-review --support-grips-only`로 아야·버터·코미·시온의 1인칭 및 외부 시점 대기/공격 화면 16장과 손바닥 파지를 확인했다. 화면 모음은 `artifacts/arm-isolation/game-gallery-contact.png`이다.

단일 실행 파일: `release/TSB-Multiplayer-ArmRigFix.exe`.

단일 EXE의 `--verify-only`도 종료 코드 0으로 통과했다. 포함 파일 1,330개의 추출·무결성을 확인했으며, 실행 파일 해시와 전체 결과는 `artifacts/arm-isolation/delivery-report.json`에 기록했다.
