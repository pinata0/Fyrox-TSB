저폴리곤 원본 전달분 (2026-11-20 / 적용 작업 2026-11-21)

temp에서 받은 .blend 32개를 캐릭터별 폴더에 옮겼다.
manifest.json에 원래 파일명, 사용 캐릭터/종류, 새 경로, SHA-256을 기록했다.
이 폴더의 파일은 전달받은 원본 그대로이며, 게임용 리깅/애니메이션은
assets/characters/<캐릭터>.blend와 .fbx에 들어 있다.

적용 내용
- 캐릭터 몸체: 원본 리메시 약 1만 삼각형. 별도 제작된 팔과 겹치는 원래 팔은 제거.
- 별도 손: 한 손당 약 1,500 삼각형. 기존 손가락 뼈와 그립 정보를 유지.
- 일반 무기: 약 3,000 삼각형. 아야의 별도 변형 칼날 906개, 우이의 축 60개는 추가 유지.
- 개구리/세바스찬: 각각 약 5,000 삼각형, 이드 의자 약 6,000개, 나타 약 3,000개.
- 캐릭터 전체 15종(손/소매/동료/의자 포함): 1,899,063 -> 236,999 삼각형.
  일반 캐릭터는 약 1.4만개이며, 동료가 포함된 캐릭터는 약 1.9만~2.4만개다.
- 기존 뼈의 이름/계층/기준 자세와 애니메이션 키를 보존하고 스킨 가중치를 전이.
- 표정 23종, 깜빡임 노멀, UV별 표정 텍스처, 볼 변형 마스크를 새 토폴로지에 맞게 전이.
- 의자 패널/회전 클로버/우산 변형 부품의 이름과 회전축을 유지.
- 세바스찬의 기존 머리 전진 보정과 1인칭 팔 메시도 새 모델에 반영.

검증/백업 자료
artifacts/lowpoly-import/before : 적용 전 게임 에셋의 독립 복사본
artifacts/lowpoly-import/characters : 검증용 에셋
artifacts/lowpoly-import/rig-validation.json : 기존 뼈/애니메이션 일치 검사 및 메시별 삼각형 수
artifacts/lowpoly-import/triangle-summary.json : 캐릭터별 전후 삼각형 수
artifacts/lowpoly-import/published.json : 반영 파일과 해시
artifacts/lowpoly-qa-*.log : 실제 게임의 GPU 회귀 검사
artifacts/perf-lowpoly-*.log : 최종 에셋의 릴리스 성능 측정

작업 스크립트는 scripts/*lowpoly*.py와 scripts/review_lowpoly_assets.ps1에 있다.
export_meshy_morphs.py와 build_chloe_view_hands.py가 파생 메시 데이터를 생성한다.
원본 리메시 파일만 FBX로 내보내 덮어쓰면 리그와 표정/그립 데이터가 유실되므로,
다음 수정은 게임용 .blend에서 시작하거나 같은 전이/파생 데이터 생성 과정을 거쳐야 한다.
