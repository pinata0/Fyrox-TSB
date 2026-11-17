# 원작 성격·종족·포지션 마커

2026-11-13 적용. [Prydwen 트릭컬 도감](https://www.prydwen.gg/trickcal/characters/)의 성격·종족·포지션 필터가 사용하는 원작 마커 15개를 확보했다. 한국어 대응은 기존 로스터와 [가게더 한국 서버 도감](https://gagether.wiki/ko/games/trickcal/characters?region=kr)을 대조했다.

| 분류 | 게임 내부 순서 | 원본 파일 이름 |
|---|---|---|
| 성격 | 순수·냉정·광기·활발·우울 | element_innocence·element_composed·element_madness·element_vivacious·element_depressed |
| 종족 | 요정·수인·엘프·정령·유령·용족·마녀 | race_sprites·race_werebeast·race_elves·race_elementals·race_phantoms·race_dragons·race_witches |
| 포지션 | 전열·중열·후열 | position_front·position_middle·position_back |

원본은 `https://cdn.prydwen.gg/images/trickcal/icons/`의 40×40 WEBP다. 모든 파일에 이미 배경을 분리한 알파 채널이 있어, 이를 그대로 보존한 RGBA PNG로 변환했다. 원작의 둥근 색상 배지와 내부 문양은 유지한다. 다시 그리기·색 변경·크롭·생성형 복원은 하지 않았다. 광기·활발 원본에는 모서리에 1/255의 미세한 알파 표본이 있어 이 안티앨리어싱도 보존한다.

- 게임 에셋: `assets/ui/markers/*.png`
- 다운로드 원본: `assets/references/trait-markers/*.webp`
- 개별 URL·SHA256·해상도·알파 검사: `assets/ui/markers/sources.json`
- 재현: Pillow가 있는 Python으로 `python scripts/fetch_trait_markers.py`. 이미 받은 원본은 재사용한다. 변환 후 픽셀 일치와 투명 배경을 검사한다.
- 트릭컬 원작 마커의 권리는 EPID Games에 있다. 이 출처 기록은 별도의 이용 허가를 의미하지 않는다.

선택창의 캐릭터 특성 세 항목, 성격별 팀 인원, 전열·중열·후열 제목에 마커를 적용했다. 원작 마커가 낯선 경우에도 읽을 수 있도록 이름과 인원은 유지한다. 캐릭터 카드 15개와 팀 슬롯 6개에는 성격 배지를 표시하며, 선택 불가 카드는 초상화와 함께 배지도 흐려진다. 선택창을 닫으면 배지도 함께 숨긴다. 1280×800 기준 좌표에 기존 화면 배율을 적용하고 이미지의 가로세로 비율을 1:1로 유지한다.

## 검증

- `cargo check --locked --bin tsb`, fmt, Clippy `--all-targets -- -D warnings` 통과.
- 기존 `target/debug/tsb.exe`의 접근 충돌을 피하여 `cargo rustc --locked --bin tsb -- -o artifacts/trait-markers/tsb-markers.exe`로 별도 실행 파일 빌드 성공.
- `--selection-review`: 1920×1200에서 15명·30장, `--selection-review --hud-review-small`: 960×640에서 15명·30장. 마커 리소스 로드 실패와 패닉 없이 정상 종료했다. 비비·아야·시온 화면에서 서로 다른 성격·종족·포지션 대응과 크기·정렬을 시각 확인했다.
- 선택창 검증 모드는 독립 무대만 사용하므로 성 맵 장식 로드를 생략한다. 일반 게임 실행의 맵 로드는 유지한다.
- `artifacts/trait-markers/normal/`, `small/`, `validation.json`에 캡처 60장과 파일 해시·크기·알파 검사 결과를 보관한다.
- 마지막 전체 테스트에서는 라이브러리 56개가 통과했으나, 별도 성 맵 작업의 `tests/castle.rs`에서 JSON 정수와 실수 배열 비교 1개가 실패해 후속 통합 테스트 전에 중단됐다. `tests-final.log`에 기록했으며 마커 작업에서 맵·전투 판정은 수정하지 않았다.
