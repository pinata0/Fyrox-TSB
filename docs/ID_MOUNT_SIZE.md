# 이드 초승달 탑승물 크기 수정

이드의 머리카락 뒤로 가려지던 초승달 장치를 확대해 몸을 감싸는 좌석과 등받이가 드러나게 했다. 폭은 0.93m에서 1.58m, 전체 높이는 1.22m에서 2.07m로 약 1.7배 커졌다.

앞쪽 테두리는 무릎 높이에 맞춰 상체와 얼굴을 가리지 않도록 낮췄다. 장치를 뒤로 옮겨 발이 앞쪽으로 나오게 하고, 나타는 오른쪽 위로 옮겨 커진 외곽과 겹치지 않게 했다. 이드 본체, 얼굴 모프, 손 리그, 전투 수치와 피격 판정은 변경하지 않았다.

동시에 진행된 얼굴 메시 갱신과 합친 뒤에도 초승달·나타 정점이 검증된 후보와 동일한지 확인했다. 실행 검사에서 발견한 초기화 전 UI 접근은 `Game`이 초기화 완료 후에만 런타임을 공개하도록 수정했다. 폴더를 텍스처 파일로 잘못 저장하는 문제도 수정하고, 기존 이미지 이름·패킹 정보를 이용해 파일 연결을 복구했다. 관련 스크립트는 `scripts/repair_character_texture_links.py`이며, 복구 전 모델은 `artifacts/id-mount-size/texture-repair-before/`에 보관한다.

## 파일

- `assets/characters/id.blend`, `id.fbx`: 수정된 편집 원본과 게임 모델.
- `assets/characters/manifest.json`: 크기 변경 정보와 파일 해시.
- `scripts/resize_id_mount.py`: 원본 백업에서 동일한 수정안을 만드는 스크립트.
- `artifacts/id-mount-size/before/`: 수정 전 원본.
- `artifacts/id-mount-size/after-front.png`, `after-side.png`, `after-three-quarter.png`, `after-back.png`: Blender 검토 이미지.
- `artifacts/id-mount-size/asset-verification.json`: 529프레임 본체 모션 보존, 11개 클립, 스킨·표정 보존과 독립 FBX 재가져오기 검사.

## 재실행

Blender의 `--background --factory-startup --python-exit-code 1 --python` 뒤에 아래 스크립트를 지정한다. 크기 수정은 손 메시 후처리까지 끝난 모델을 기준으로 적용한다.

1. `scripts/resize_id_mount.py`: 보존된 수정 전 모델에서 후보를 다시 생성한다.
2. `artifacts/id-mount-size/verify.py`: 본체·모션 보존과 FBX를 검사한다.
3. `scripts/export_meshy_morphs.py -- artifacts/id-mount-size/candidate`: FBX 모프 연결 데이터를 추출한다.
4. 렌더를 확인한 뒤 `scripts/resize_id_mount.py -- --publish`: 검증된 후보를 적용한다. 파일 해시와 모프 데이터가 맞지 않거나 현재 모델이 별도로 변경되었으면 중단한다.

전체 캐릭터를 새로 생성한 경우에는 기존 `before` 백업을 다른 이름으로 보존하고 새 기준 모델을 백업한 다음 적용해야 한다. 기존 백업으로 재실행하면 해당 백업 당시의 본체를 기준으로 후보가 생성된다.

실행 파일은 `release/TSB-Multiplayer-IdMount.exe`이며, 패키지와 게임 화면 검사 결과는 `artifacts/id-mount-size/validation-report.json`에 기록한다.
