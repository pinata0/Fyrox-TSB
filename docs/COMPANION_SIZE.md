# 세바스티안과 에루 크기 수정

저장된 원작 이미지와 현재 게임 모델을 대조해 두 동료의 크기를 조정했다.

클로에가 세바스티안의 양 볼을 받치는 원작 자세는 후속 [받치기 자세 수정](CHLOE_CARRY_POSE.md)에 적용했다. 해당 수정판이 크기 조정과 자세 교정을 모두 포함한다.

- 세바스티안: 기존의 2.4배. 모델 높이 1.25 → 3.00m, 폭 0.93 → 2.23m. 클로에 뒤쪽에서 발을 바닥 가까이에 두고 얼굴·양 볼·하트 장식이 클로에 머리 위로 드러나게 배치했다.
- 에루: 기존의 1.55배. 모델 높이 0.66 → 1.02m, 폭 0.98 → 1.52m. 원작처럼 우이의 모자보다 넓은 탑승물 실루엣으로 맞췄다. 우이의 탑승 높이는 모션에서 0.30m 올렸다.
- 확대된 팔·발이 기존 모션 중 바닥 아래로 들어가지 않도록 매 프레임 동료 루트 높이를 보정했다. 우이는 에루의 보정 높이를 함께 따른다.
- 선택창에서 두 캐릭터의 카메라 높이와 거리를 조정했다.

비율의 근거는 `artifacts/tripo-sebastian-original-style/references/chloe-official-jp-basic.jpg`, `assets/references/chloe-fullbody.png`, `assets/references/ui-fullbody.png`이다. 2D 원작을 현재 3D 모델에 맞춘 시각적 조정이며 원작의 공식 신장 수치는 아니다.

캐릭터 본체의 정점·표정 모프·스킨 가중치·UV·손 리그와 전투 판정은 유지했다. 동료 메시와 대응 본을 함께 확대했으며, 우이의 위치는 기본 메시를 변경하지 않고 애니메이션으로 이동해 기존 얼굴·볼 상호작용 좌표를 보존했다.

## 재현과 검증

Blender의 `--background --factory-startup --python-exit-code 1 --python` 옵션으로 실행한다.

1. `scripts/resize_companions.py`: 최초 백업에서 후보 생성과 전후·정면·측면 렌더. 다시 실행해도 크기가 누적되지 않는다.
2. `scripts/export_meshy_morphs.py -- artifacts/companion-size/candidate chloe ui`: 게임의 모프 데이터 추출.
3. `artifacts/companion-size/verify.py`: 두 캐릭터의 529프레임, 11개 클립, 본체·표정·가중치 보존, 바닥 높이, FBX 재가져오기와 텍스처 검사.
4. `scripts/resize_companions.py -- --publish`: 검증한 파일 해시와 현재 에셋을 대조한 뒤 적용. 독립적인 수정이 발견되면 덮어쓰지 않는다.

편집 원본·게임 모델은 `assets/characters/chloe.blend`, `chloe.fbx`, `ui.blend`, `ui.fbx`에 반영한다. 수정 전 파일은 `artifacts/companion-size/before/`, 상세 수치와 검사 결과는 같은 작업 폴더의 `resize.json`, `verification.json`에 보관한다. 전체 모델 재생성 시에는 기존 백업을 보존한 뒤 새 기준 파일로 후처리를 적용해야 한다.

release 빌드와 실제 게임의 전체 15명 전후면 모션 캡처 30장, 최종 선택창·볼 상호작용 캡처 21장을 확인했다. 두 동료의 게임 화면은 `artifacts/companion-size/captures/`에 보관한다. 단일 실행 파일은 `release/TSB-Multiplayer-CompanionSize.exe`이다.
