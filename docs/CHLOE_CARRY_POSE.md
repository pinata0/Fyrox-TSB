# 클로에의 세바스티안 받치기 자세

앞선 크기 수정에서 빠졌던 원작의 기본 조립 자세를 적용했다. 세바스티안의 머리는 클로에의 머리 위에 놓이며, 클로에는 양팔을 올려 손바닥으로 세바스티안의 양 볼 아래를 받친다.

기준 이미지는 `artifacts/tripo-sebastian-original-style/references/chloe-official-jp-basic.jpg`와 `assets/references/chloe-fullbody.png`이다.

## 2026-11-16: 양팔을 곧게 펴서 받치기

양팔이 길게 꺾여 팔꿈치가 늘어져 보이던 자세를 수정했다. 소매 길이를 볼까지의 거리에 맞추고, 어깨·팔꿈치·손목이 일직선으로 이어지게 했다. 손바닥은 기존과 같이 세바스티안 양 볼 아래를 받친다. 손·손가락 크기, 얼굴·몸·세바스티안 메시 및 팔 외의 애니메이션(Spawn 포함)은 보존한다.

`scripts/straighten_chloe_carry.py`는 **현재 적용된 받치기 모델**을 `artifacts/chloe-straight-arms/before/`에 한 번 백업한 뒤 후보를 만든다. 예전 크기/받치기 백업에서 다시 생성하지 않으므로 이후 추가된 몸 동작이 유지된다. 빠른 동작에서도 손이 볼을 따라가도록 기존 30 fps 타임라인의 0.25프레임마다 자세를 굽고, FBX도 같은 간격으로 내보낸다.

Blender `--background --factory-startup --python-exit-code 1 --python`으로 다음 순서대로 실행한다.

1. `scripts/straighten_chloe_carry.py`
2. `scripts/export_meshy_morphs.py -- artifacts/chloe-straight-arms/candidate chloe`
3. `scripts/verify_chloe_straight_carry.py`
4. `scripts/straighten_chloe_carry.py -- --publish`

검사는 팔꿈치 직선 정렬·손바닥 접촉·손 크기·프레임 사이 보간, 팔 이외의 애니메이션과 본체 보존, FBX 재가져오기를 확인한다. 수정 모델은 `assets/characters/chloe.blend`, `chloe.fbx`, `chloe-hands.json`에 반영하며, 렌더와 수치 검사는 `artifacts/chloe-straight-arms/`에 보관한다.

529프레임 구간에 구운 2,113개 표본과 그 사이 2,112개 표본을 검사했다. 게임용 FBX의 손 접촉 오차는 보간 포함 최대 0.718 mm, 팔꿈치 직선 이탈은 0.001 mm 미만이다. 얼굴·몸·세바스티안의 형상/UV/가중치/표정과 팔 외의 기존 애니메이션은 동일하다. 편집 원본은 기존 몸 동작의 비선형 곡선을 유지하므로 원본 보간 접촉 오차는 최대 8.15 mm이며, 게임용 FBX에서는 모든 본을 같은 간격으로 구워 위의 0.718 mm 이내로 유지한다.

새 실행 파일은 `release/TSB-Multiplayer-ChloeStraightArms.exe`이다. 기존 `release/TSB-Multiplayer-ChloeCarry.exe`는 이 수정 전 자세를 포함한 이전 배포본이다.

실제 게임 선택/볼 반응 화면 21장 검사(`CHEEK_REVIEW_OK`, 종료 코드 0)와 EXE 내 1,334개 파일의 해시 검사를 통과했다. 게시 에셋·검사 후보·EXE 내부 클로에 에셋의 해시가 일치한다. 최종 결과는 `artifacts/chloe-straight-arms/validation-report.json`, 클로에 게임 캡처 7장은 같은 폴더의 `captures/`에 있다.

## 최초 받치기 자세 적용

- 세바스티안 상부를 앞으로 이동해 머리가 클로에 정수리 위에 놓이게 했다. 몸통과 긴 인형 팔은 뒤에 유지하며 옆 술 장식도 함께 정렬했다.
- 짧았던 클로에의 팔 리그와 닫힌 소매 메시를 받치기 자세에 맞춰 늘렸다. 노란 소매와 흰 소매 끝을 적용했다. 손 크기와 손가락 리그는 유지했다.
- 양 볼의 실제 표면에서 아래쪽 접촉점을 구하고, 매 프레임 해당 표면을 따라 양팔 IK를 풀어 손바닥을 위로 향하게 했다. 대기·이동·공격·선택 등 11개 클립 529프레임에 구웠다.
- 얼굴·몸통·다리의 메시, 표정 모프, UV, 스킨 가중치와 해당 본의 기존 애니메이션은 보존했다. 에루 크기 수정은 유지된다.

## 재현

Blender `--background --factory-startup --python-exit-code 1 --python`으로 실행한다.

1. `scripts/pose_chloe_carry.py`: 최초 백업에서 후보를 만든다.
2. `scripts/export_meshy_morphs.py -- artifacts/chloe-carry/candidate chloe`: 모프 연결 데이터를 추출한다.
3. `artifacts/chloe-carry/verify.py`: 529프레임의 양손 접촉·손바닥 방향, 얼굴과 본체 보존, FBX 재가져오기를 검사한다.
4. `scripts/pose_chloe_carry.py -- --publish`: 파일 해시를 확인하고 현재 에셋에 적용한다.

현재 편집 원본과 런타임 파일은 `assets/characters/chloe.blend`, `chloe.fbx`, `chloe.morphs.json`, `chloe-hands.json`이다. 원본 백업과 렌더·검사 결과는 `artifacts/chloe-carry/`에 보관한다. 이전 크기 수정 스크립트를 다시 적용한 경우 이 자세 후처리까지 적용해야 한다.

받치기 자세 수정판은 `release/TSB-Multiplayer-ChloeCarry.exe`이다.

## 검증 결과

- 후보 모델 529프레임 전체에서 양손 접촉 오차는 최대 0.0008 mm이며, 24개 표정 모프와 FBX 재가져오기 검사를 통과했다.
- 실제 게임에서 볼·선택 화면 21장과 전체 캐릭터 무기·월드 동작 102장을 검사했다. `CHEEK_REVIEW_OK`, `WEAPON_REVIEW_OK`, `RIGHT_HAND_ROSTER_OK`를 확인했다.
- 단일 EXE에 포함된 1,332개 파일을 모두 해시 검사했다. 결과는 `artifacts/chloe-carry/validation-report.json`에 기록했다.
- EXE SHA-256: `825b9ce7b5287b926fa3fb01c6bdf9c66d5bcfa971bb812ecc53395d9fbc3d09`.

현재 작업 폴더에는 이 자세 수정을 적용한 뒤 별도 작업에서 추가한 Spawn 동작도 유지되어 있다. 이 EXE는 게임 검사를 통과한 받치기 자세 스냅샷이며, 이후 추가된 Spawn 동작은 포함하지 않는다. 적용 이력은 `artifacts/chloe-carry/source-after-publication.json`에 기록했다.
