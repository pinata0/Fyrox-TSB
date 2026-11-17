# 이드 탑승 자세 수정

확대한 초승달 좌석에 맞춰 골반을 뒤쪽 쿠션에 붙이고, 무릎을 접어 두 발을 내부 받침 위에 올렸다. 상체는 약간 기대고 대기·이동·점프 중 손은 무릎 위에 놓는다. 탑승물 크기는 이전 수정판을 유지한다.

걷기용 골반 움직임을 그대로 쓰면서 좌석에서 떠오르던 현상도 줄였다. 골반이 탑승물 움직임을 따라가게 하고 호흡·반동에 의한 상대 이동은 최대 1.5cm로 제한한다. 공격·스킬·궁극기·춤·선택 모션의 팔 동작은 유지하며, 사망 모션에서는 착석 보정을 점차 풀어 기존 마지막 자세로 이어진다.

## 적용 파일

- `assets/characters/id.blend`, `id.fbx`, `id.morphs.json`: 편집 원본과 게임 에셋. 메시·표정·스킨 가중치와 초승달·나타 애니메이션은 보존했다.
- `assets/characters/manifest.json`: 이드의 해시와 `seated_pose` 기록.
- `scripts/pose_id_rider.py`: 보존된 기준 모델에서 11개 클립의 529프레임을 다시 베이크한다.
- `artifacts/id-seated-pose/before/`, `before-publish/`: 작업 시작 시점 및 반영 직전의 원본 백업.

## 재생성과 검증

Blender 5.3의 `--background --factory-startup --python-exit-code 1 --python` 옵션으로 실행한다.

1. `scripts/pose_id_rider.py -- --source C:/Users/user/projects/Fyrox-TSB/artifacts/id-seated-pose/before-publish/id.blend`: 후보 생성. 새 모델에 적용할 때는 먼저 그 모델을 별도로 백업해 `--source`에 지정한다.
2. `scripts/export_meshy_morphs.py -- artifacts/id-seated-pose/candidate id`: FBX의 모프 연결 데이터 추출.
3. `artifacts/id-seated-pose/check_pose.py`: 형상·가중치·표정 보존, 54본, 11클립, 반복 경계, 좌석 접촉, FBX 재가져오기 검사.
4. 렌더를 확인하고 `scripts/pose_id_rider.py -- --publish`: 검증한 후보를 반영한다. 현재 원본이 후보 생성 이후 변경되었으면 중단한다.

`after-side.png`, `after-three-quarter.png`는 기본 자세, `action-*.png`는 이동·점프·공격·궁극기·춤·선택 중 자세다. `pose-verification.json`에 에셋 검사, `validation-report.json`에 패키지와 게임 GPU 실행 검사를 기록한다.

실행 파일은 `release/TSB-Multiplayer-IdSeated.exe`다. 이전 탑승물 확대판의 검증된 런타임에 이번 이드 에셋을 적용한 독립 패키지다.
