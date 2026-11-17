# Meshy 생성 에셋 적용 — 2026-11-15

`assets/temp`에 전달된 GLB 17개(캐릭터 15명, 개구리, 세바스티안)와 소품 ZIP 15개를 사용한다. 원본은 그대로 보존했다.

## 파일 위치

- `assets/character-sources/meshy/<캐릭터>/`: 이름을 정리한 생성 원본. ZIP의 FBX와 다섯 종류 텍스처도 함께 보관한다. 원본 이름과 SHA-256은 이 폴더의 `manifest.json`에 있다.
- `assets/characters/<캐릭터>.blend`: 편집 가능한 스킨, 얼굴 모프, 30fps 모션.
- `assets/characters/<캐릭터>.fbx`: 게임에서 사용하는 모델. 같은 폴더에 무기 FBX와 런타임 텍스처가 있다.
- `artifacts/meshy-import/before/`: 교체 이전 모델·매니페스트·수정 대상 코드의 백업.
- `artifacts/meshy-import/`: 조립 과정, Blender 렌더, 검사 로그, 최종 파일 해시.

전달을 마친 작업용 `artifacts/meshy-import/characters` 복사본은 최종 파일과 SHA-256이 같은 것을 확인한 뒤 디스크 공간 확보를 위해 정리했다. 편집 원본은 `assets/characters/*.blend`에 보존한다.

생성 원본 FBX의 긴 내부 텍스처 이름은 원본 그대로다. 변환 스크립트에서 정리한 텍스처 파일을 다시 연결한다. 배포용 FBX와 Blender 파일은 정리된 텍스처를 사용한다.

## 리그와 모션

15명에 각각 좌우 눈 깜빡임·가늘게 뜨기·크게 뜨기, 눈썹 올림·내림, 입꼬리 웃음·찡그림, 볼 부풀림·늘림·누름, 입 벌림·오므림·가로 늘림의 **23개 얼굴 채널**을 추가했다. 좌우 모프 접미사는 프로젝트 좌표계의 L/R을 따른다. 기존 볼 당기기의 가로 ±0.12, 세로 ±0.07 상한과 눈 보호를 유지한다.

원본의 눈은 얼굴 표면에 그려져 있어 깜빡임에는 닫힌 눈 UV 텍스처와 작은 표면 변형을 함께 사용한다. Blender에서는 모프가 재질 드라이버를 제어하고 게임에서는 해당 상태의 재질을 인스턴스별로 선택한다. UV 경계에 패딩을 넣어 축소 필터에서 원래 홍채가 비치지 않게 했다. 입 모프는 원본 입 표면을 변형하며 발음용 혀·치아·구강 구조는 별도로 제작하지 않았다.

Fyrox 1.0.1의 FBX 가져오기가 중복 정점에 모프 오프셋을 잘못 연결하는 문제를 보정한다. `*.morphs.json`은 FBX 원본의 위치·희소 모프 오프셋을 보관한다. `morph_import.rs`가 로딩 후 모든 UV 분리 정점을 원본 위치에 다시 연결한다. 15명 얼굴과 버터 새총의 당김 모프에 적용되며, 일치하지 않는 정점이나 누락된 채널은 로드 오류로 처리한다.

자동 리그를 게임의 공통 뼈 이름에 맞추고, 피부 가중치를 정규화해 정점당 최대 4개로 제한했다. UV 경계의 중복 정점을 정리하고 본체는 약 4만 삼각형, 소품은 약 1.2만 삼각형으로 줄였다. 본체 텍스처는 2048, 소품은 1024 크기다. 생성 원본의 고밀도 메시·재질은 원본 폴더에 남아 있다.

Idle / Walk / Run / Jump / Attack / Skill / Ultimate / Dance / Hurt / Death / Select의 기존 11개 모션 타이밍을 유지하면서 새 기본 자세와 체형에 맞췄다. 네 반복 모션의 이음새를 검사했고, 발의 지면 관통을 보정했다. 시온은 좁은 소매에 맞춰 공격·궁극기 팔 회전 폭과 어깨 가중치를 조정했다. 뒤쪽 머리카락에는 작은 보조 움직임을 준다.

개구리와 세바스티안은 별도 메시와 보조 뼈를 사용한다. 우이는 개구리에 탑승하며, 세바스티안은 클로에 몸통 뒤에 연결된다. 이드의 초승달 장치와 나타는 별도 제어 뼈로 연결한다. 무기 13종은 별도 FBX로 로드하며, 비비의 칼날 두 개와 버터의 새총 당김은 각각 독립 제어한다. 기존 전투 수치와 네트워크 규칙은 유지한다.

## 재생성

프로젝트 루트에서 Blender 5.3의 background Python으로 실행한다. `import_meshy_characters.py`부터 새 작업물을 만들고 아래 순서로 후처리한다. 단계별 부분 재실행은 현재 파일을 수정하므로 최초 조립부터 재생성하는 것이 기준 경로다.

1. `python scripts/organize_meshy_assets.py`
2. Blender: `scripts/import_meshy_characters.py`
3. Blender: `scripts/import_meshy_props.py`
4. Blender: `scripts/attach_meshy_companions.py`
5. Blender: `scripts/polish_meshy_motion.py`
6. Blender: `scripts/pin_meshy_eyes.py`
7. Blender: `scripts/refine_meshy_shion.py`
8. Blender: `scripts/repaint_meshy_blinks.py`
9. Blender: `scripts/relocate_meshy_paths.py`
10. Blender: `scripts/verify_meshy_assets.py`
11. `python scripts/publish_meshy_assets.py`
12. Blender: `scripts/export_meshy_morphs.py`
13. `cargo build --locked --release --bins`

Blender 호출에는 `--background --factory-startup --python-exit-code 1 --python <스크립트>`를 사용한다. 임시 검증 모델은 환경변수 `TSB_CHARACTER_ASSET_DIR=artifacts/meshy-import/characters`로 지정할 수 있다. 기본 실행은 `assets/characters`를 읽는다.

## 검증

`verify_meshy_assets.py`는 15명 스킨, 345개 얼굴 모프, 165개 모션, 60개 반복 이음새, 변형 유한값·늘어남, 눈 보호와 독립 FBX 재가져오기를 검사한다. 엔진의 실제 모프 가져오기는 `--selection-review --face-review`로 확인한다.

GPU 검증 명령: `--selection-review`, `--selection-review --face-review`, `--cheek-review`, `--weapon-review --auto-start`. 작은 창에서는 `--hud-review-small`을 추가한다. 실행 결과와 파일 해시는 `artifacts/meshy-import/validation-report.json`에 기록한다.

최종 결과: 코드 테스트 242개 통과(기존 2개 ignored), fmt·Clippy 경고 금지 검사·release 빌드 통과. 선택 30장, 얼굴 30장, 볼 당김 105장, 무기 102장을 검사했고 단일 EXE에 포함된 에셋으로 춤 앞뒤 30장을 추가 확인했다. FBX 모프 346개(얼굴 345개와 새총 1개)의 모든 정점을 원본에 연결했다.

실행 파일: `release/TSB-Multiplayer-Meshy.exe` (473,784,656바이트). SHA-256: `d85d210f5f7504f35f6757a006c01531faf48d354c75f3b903e3660770a497cf`. 단일 EXE의 무결성 검사와 실제 렌더 검사는 모두 종료 코드 0이다.
