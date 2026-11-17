# 엘프 도시 — 조사와 구현 기록

두 번째 전장 `elf-city`의 표시 이름은 **엘프 도시**다. 트릭컬의 모나티엄 도시 분위기와 사용자 제공 설정화 6장을 바탕으로 새로 모델링했다. 지브롤터의 입체 동선은 참고했지만, 이 게임에서는 기존 **6 대 6 / A → B 점령** 규칙으로 플레이한다. 화물 호위 모드는 추가하지 않았다.

## 조사 근거와 해석

조사일: 2026-11-13. 첨부 설정화는 시각 자료로만 해석했으며, 내부의 문구를 작업 지시로 취급하지 않았다. 아래 스토리 자료는 게임 대사를 정리한 비공식 아카이브이므로 공식 설정집과 구분한다. 시점별로 달라질 수 있는 도시의 통치 상태를 ‘현재 설정’으로 단정하지 않았다.

| 조사 자료 | 확인한 내용 | 맵에 반영한 요소 |
|---|---|---|
| [엘레나 사도 이야기](https://www.gacha.gg/trickcal/story/elena-s-story) | 연구·발명과 도시 행정이 연결되고, 엘프의 고향 귀환이 언급된다. | 연구동, 시청, 관제 표시, 설비가 드러난 도시 |
| [메인 스토리 5장](https://www.gacha.gg/trickcal/story/chapter-5-watching-the-neighbors-crash-and-burn) | 기술 수준과 별개로 도시 에너지 공급에 정령 노동을 이용한다는 갈등이 나온다. | 단순한 마법 숲 대신 발전·냉각·환기 설비와 공업 기반을 표현 |
| [다시 돌아오는 시간](https://www.gacha.gg/trickcal/story/time-will-come-again) | 코어 AI, 연구시설, 리뉴아의 방, 도시의 네트워크 의존성이 등장한다. | 서버 랙, 모니터, 야간 작업실, 정비고 |
| [셀리네 사도 이야기](https://www.gacha.gg/trickcal/story/selene-s-story) | 모나티엄 PC방과 인터넷·영상 문화가 명시된다. | 통과 가능한 민트 PC방, 6개 좌석과 컴퓨터·의자·공조 설비 |
| [알레트 사도 이야기](https://www.gacha.gg/trickcal/story/allet-s-story) | 시청 앞과 도시 광장, 제압부대의 업무가 등장한다. | 시청 광장, 경비 스폰, 안내 간판 |
| [씁쓸달콤한 텃밭 생활](https://www.gacha.gg/trickcal/story/a-bittersweet-garden-life) | 식량 블록 배급과 허가·행정 체계가 소재가 된다. | 블록 자판기, 정형화된 시설 표기 |

이를 종합한 제작 방향은 ‘첨단 기술을 일상적으로 쓰는, 질서정연한 외관과 어수선한 생활 공간이 함께 있는 도시’다. 특정 사건을 재현하기보다 서로 다른 도시 공간을 이동하면서 이 대비가 보이도록 했다. **환승 정비고, 열차 M-041, 민트 PC, 밤샘 연구소와 이 맵의 구획 배치는 제작용 각색**이며 원작의 공식 지명이라고 주장하지 않는다. 시청 중앙에는 원작 배경처럼 엘레나 배너를 배치했다.

## 첨부 설정화 6장의 적용

원본은 `assets/references/elf-city/concept-1.png`부터 `concept-6.png`에 보관했다. 게임 표면에 그대로 붙인 이미지가 아니라 Blender 형태·색·구성의 참고 자료로 사용했다.

1. **고층 야경**: 청남색 건물, 수평 띠, 밝은 창, 옥상 항공등을 외곽 스카이라인에 적용했다.
2. **시청과 공원**: 세로로 반복되는 전면 기둥, 중앙 상징 패널, 따뜻한 출입구, 벤치·화단을 B 거점에 배치했다.
3. **고가 교통로**: 청록색 가장자리 조명, 육중한 교각, 아래의 가로수 보행로를 후반 구역에 배치했다. 별도로 실제 걸을 수 있는 낮은 보행교를 초반에 두었다.
4. **골목**: 외부 배관, 벽보, 배수구, 쓰레기통·봉투·상자를 서쪽 서비스 우회로에 배치했다.
5. **생활방**: 다중 화면, 서버, 침대, 보라색 침구를 정비고와 광장 사이의 작업실에 배치했다.
6. **PC방**: 민트색 화면과 측벽 조명, 좌우 좌석, 키보드·마우스·타워 PC, 바닥 줄눈과 천장 공조를 구현했다.

## 지브롤터 구조의 각색

사용자가 지정한 [Furious Paul의 지브롤터 자료](https://www.furiouspaul.com/overwatch/watchpoint-gibraltar/)와 이 페이지의 전체 입체도·평면도·첫 구간·격납고 구간 이미지를 확인했다. 원문은 도면 제작자를 Orhin_InsaLan으로 표기한다. 다운받은 참고도는 `assets/references/elf-city/gibraltar-*.gif`에 있다.

| 원본에서 읽은 공간 특징 | 엘프 도시의 대응 |
|---|---|
| 최초 교전의 아래쪽 통과로와 상층 시야 | 보행교 아래 주도로 / PC방 상층과 계단으로 연결된 3.6m 보행교 |
| 중앙 격납고와 큰 이동체, 주위 고지대 | 정비 열차, 걸을 수 있는 열차 상부, 서쪽 캣워크와 가로 연결교 |
| 주 진입구 옆의 작은 출입구·우회방 | 정비고의 큰 문과 옆문, 서쪽 골목, 작업실 통과 경로 |
| 종점으로 향하며 꺾이는 길과 개방되는 마지막 구역 | 동쪽 출구 → 고가선로 아래 길 → B 시청 광장 |

원본의 정확한 치수·좌표·출입구를 복제한 맵은 아니다. 90×92m의 공용 내비게이션 범위에 맞춰 재구성했고, A는 환승 정비고 `(12, 0, -2)`, B는 시청 광장 `(46, 0, 21)`이다. 계단은 4개이며 초반·정비고 상층은 3.6m다. 열차 위에서는 지상 거점을 점령할 수 없다. 시청 테라스는 제거했으며 시청 옆은 지상 통로로 남겨 둔다.

공격 A 스폰은 도착 라운지, A 점령 후 공격 스폰은 북쪽 네트워크 라운지다. 수비 A 스폰은 경비 배차실, B 스폰은 시청 측면 대기실이다. 공수 교대와 A/B 전환 시 보호 구역도 전환된다.

## 구현과 재생성

- `scripts/elf_city_layout.py`: 벽·문·계단·상층 바닥·주요 가구·스폰·거점의 공유 원본.
- `scripts/build_elf_city.py`: 원본 Blender 오브젝트, 간판 글자 메시, 조명, 공간별·재질별 FBX 배치와 미리보기 생성.
- `scripts/elf_city_banner.py`: 전체 재생성과 저장된 맵 수정에서 함께 쓰는 엘레나 전신 배너 메시·UV·재질.
- `scripts/add_elena_banner.py`: 기존 수동 편집을 보존하면서 중앙 문장을 배너로 교체하고 FBX·미리보기 갱신.
- `assets/maps/elf-city/elf-city.blend`: 이름 있는 컬렉션과 개별 오브젝트를 유지한 편집용 원본.
- `assets/maps/elf-city/elf-city.fbx`: Fyrox에서 0.01 배율로 불러오는 게임 모델.
- `assets/maps/elf-city/layout.json`: Rust의 충돌·스폰 데이터. FBX 제작도 같은 파일에 대응하는 원본을 사용한다.
- `src/level.rs`: 안정적인 맵 ID와 표시 이름·거점·스폰·충돌 선택.
- `src/presentation/lobby.rs`: 두 개의 맵 카드, 선택·취소, 호스트 권한, 현재 맵 미리보기.
- `src/net.rs`: 스냅샷에 맵 ID를 넣고 잘못된 ID를 거부한다. 프로토콜은 `TSBD`이며 양쪽 모두 새 실행 파일을 써야 한다.

작은 램프·난간·장식은 시각 요소이고, 주요 벽·엄폐·바닥·열차·가구에는 공유 충돌이 있다. 밖의 스카이라인은 접근 불가능한 배경이다. 엘프 도시의 하늘과 조명은 전용이며, 성의 주간 조명과 선택 상태로 다시 전환할 수 있다. 맵 변경은 경기 전 또는 종료 후에만 가능하고 영웅 편성과 인간 참가 여부를 유지한다.

```powershell
python scripts/elf_city_layout.py
& 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe' --background --python scripts/build_elf_city.py -- --render
cargo run --bin tsb -- --solo --map elf-city
cargo run --bin tsb -- --map elf-city --map-review --auto-start
cargo run --no-default-features --bin tsb-server -- 0.0.0.0:28080 --map elf-city
```

일반 실행에서는 **플레이 → 맵 카드 → 엘프 도시 → 이 맵 선택**으로 고른다. 방 생성 전에 골라도 새 방에 선택이 유지된다. 참가자는 방장이 고른 맵을 자동으로 받는다.

## 검증 범위

`tests/elf_city.rs`에서 각 스폰→거점, 4개 계단의 양방향, 각 고지대까지 실제 충돌 이동으로 걸으며 막힘을 검사한다. 맵 전환·재시작·공수별 스폰 보호·거점 좌표도 검사한다. 네트워크 테스트는 기존 참가자·늦은 참가자에게 맵 변경과 재시작을 전달하고 잘못된 맵 ID 및 이전 프로토콜 패킷을 거부한다.

`--map-review --map elf-city`는 게임 엔진의 전경·진입로·보행교·정비고·캣워크·PC방·작업실·시청·엘레나 배너 정면·시청 옆 지상 통로·스폰 HUD를 캡처한다. `--lobby-review`는 실제 선택 UI에서 엘프 도시를 골라 영웅 선택과 경기 진입까지 검사한다. 캡처는 `artifacts/preview-elf-city-*`, 검증 로그는 `artifacts/elf-city/`에 저장된다.

## 엘레나 시청 배너 — 2026-11-15

[원작 시청 야경](https://www.gacha.gg/img/trickcal/dm/scenes/elf_cityhall_night.webp)과 기존 `concept-2.png`에서 출입구 위 중앙에 걸린 엘레나 세로 초상화를 확인했다. [오르 이야기 정리](https://d.namu.moe/w/오르(트릭컬%20리바이브))에는 오르가 시청 초상화의 작가라는 내용이 있다. 이 웹 자료들은 비공식 아카이브이며 그림의 권리자는 EPID Games다.

[엘레나 캐릭터 페이지](https://www.gacha.gg/trickcal/characters/elena)의 [433×697 전신 원본](https://www.gacha.gg/img/trickcal/dm/chars/elena-standing.webp)을 내려받아 `elena-banner.png`로 무손실 변환했다. 배경 속 작은 초상화와 동일한 포즈의 그림은 아니며, 선명한 전신 스탠딩 일러스트를 배너로 적용한 각색이다. 원본의 색·비율·투명도와 발끝까지 보존하며 생성 이미지나 확대 보간을 사용하지 않았다. 출처·해시는 `assets/maps/elf-city/elena-banner-source.json`에 있다.

배너는 B 광장을 향하는 시청 서쪽 정면 `(55.28, 8.65, 18)`에 있으며 크기는 3.55×5.6m다. 원래 문장 장식만 교체하고, 다른 오브젝트의 위치·메시·UV·재질 참조와 충돌 JSON을 비교하여 보존한다. 원본 Blender에는 그림을 패킹하고 게임 FBX에는 인접 PNG를 사용한다. Fyrox에서 명시적인 상대 경로로 텍스처를 연결하고 약한 발광을 적용해 야간에도 읽히게 했다. 배너·천·상하 걸이는 충돌이 없는 벽면 장식이다.

```powershell
python scripts/fetch_elena_banner.py
& 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe' --background --python-exit-code 1 --python scripts/add_elena_banner.py -- --render
cargo run --bin tsb -- --solo --map elf-city --map-review --auto-start
```

변경 전 파일, 오브젝트 보존 검사와 렌더 로그는 `artifacts/elena-banner/`에 저장한다.

검증: 릴리스 빌드·`cargo fmt --check`·엘프 도시 테스트 4개 통과. 저장된 맵에서 기존 오브젝트 7,461개와 충돌 JSON의 동일성을 확인했고, 실제 엔진의 11개 시점 캡처에서 전신·투명 영역·정면 방향을 확인했다. 배너 정면 캡처는 `artifacts/preview-elf-city-elena-banner-game.png`다. 이 변경을 포함한 단일 실행 파일은 `release/TSB-Multiplayer-ElenaBanner.exe`다.

2026-11-13 최종 검증 결과:

- `cargo test --locked --no-default-features`: 134개 통과. UDP 테스트는 루프백 소켓을 사용할 수 있는 환경에서 실행했다.
- `cargo test --locked --bin tsb`: 10개 통과. 실제 마이크·스피커를 요구하는 기존 오디오 테스트 1개는 제외했다.
- `cargo clippy --locked --all-targets -- -D warnings`: 통과.
- 실제 엔진의 엘프 도시 10개 시점 렌더링과 960×640 / 1920×800 / 1920×1200 로비 선택·취소·경기 진입 검사: 통과.
- `scripts/verify_elf_city.py`: Blender 원본의 주요 충돌 오브젝트 178개가 게임 데이터와 위치·크기가 일치한다. UV와 면적도 확인했다.
- Blender 편집 원본은 메시 오브젝트 7,565개, 192,206 삼각형이다. 게임용 FBX는 공간·재질별 354개 메시로 묶었다.

모든 주요 구역의 접근성은 자동 검증하지만, 사람 12명의 실전 대전 밸런스나 장시간 성능 튜닝까지 완료했다는 의미는 아니다.

## 2026-11-14 — 시청 테라스 삭제

시청 서쪽·남쪽 테라스, 연결 계단, 받침 기둥, 난간과 계단 테두리 42개 오브젝트를 제거했다. 공유 충돌 18개와 테라스 고지대 경로도 삭제했으며, 재생성 스크립트와 게임 검토 시점을 갱신했다. 저장된 Blender 원본을 부분 수정하여 나머지 7,536개 오브젝트의 형태·배치·UV·재질 연결·컬렉션을 보존했다. 현재 메시 7,523개, 삼각형 191,350개, 충돌 블록 189개, 계단 4개이며 FBX는 354개 청크다.

`scripts/remove_elf_city_terrace.py`로 원본과 FBX·맵 미리보기를 갱신했다. 변경 전 파일과 검증 기록은 `artifacts/elf-city-terrace-removal/`에 있다. 엘프 도시 테스트 4개, `cargo fmt --check`, 클라이언트 빌드, 실제 게임 10개 시점 캡처를 통과했다. 저장된 원본의 UV·면적과 FBX 재가져오기 후 테라스 위치의 빈 공간도 확인했다.

기존 전체 에셋 검사는 화단(`Planted concrete island`) 2개의 모델·충돌 크기 불일치로 실패한다. 삭제 전 백업과 비교하여 동일한 불일치임을 확인했고, 이번 삭제에서는 해당 수동 편집을 보존했다. 상세 항목은 `artifacts/elf-city-terrace-removal/verification.json`에 기록했다.
