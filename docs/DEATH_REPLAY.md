# 사도별 사망 모션과 녹화 데스캠

2026-11-14 적용.

## 사망 이후 화면

1. 사망 패킷을 받으면 사망 당시 사도·위치·처치자를 고정한다.
2. 사도의 앞쪽 위에서 내려다보는 탑뷰에서 해당 사도의 사망 모션을 끝까지 재생한다. 천장·벽이 있으면 카메라 거리를 줄인다.
3. 마지막 자세를 0.15초 보여준 뒤, 처치 전 최대 3초부터 처치 순간까지 처치자 1인칭 시점으로 녹화 장면을 재생한다.
4. 마지막 녹화 장면을 유지하다가 실제 서버 부활 시 즉시 내 시점으로 복귀한다. 조기 부활이나 라운드 종료도 실제 게임 상태를 우선한다.

위치·조준뿐 아니라 모든 사도의 생존 상태, 사도 종류, 애니메이션 종류/진행 시간, 공격 궤적/반동, 인사, 보호막, 스킬 표시, 투사체 위치, 거점 상태를 기록한다. 리플레이 도중 처치자의 현재 위치·사망·재선택·연결 종료는 이미 기록한 장면을 바꾸지 않는다. 현재 전투 시뮬레이션과 네트워크는 계속 진행하고, 리플레이는 렌더링할 상태만 교체한다. 부활 시간과 내 HUD는 실제 게임 상태를 따른다.

H 사도 선택·채팅·점수판은 유지한다. 다음 사도를 선택해도 탑뷰의 시신은 죽을 당시 사도를 유지한다. 접속 직후 등 기록이 없거나 유효한 처치자가 없으면 사망 자세의 탑뷰를 유지한다. 실시간 상대 관전으로 대체하지 않는다.

## 원작 동작 조사

기존 `assets/references/manifest.json`에 URL과 제작자/호스트가 기록된 원작 전투 Spine 파일 15개를 다시 분석했다. 원작 제작자는 EPID Games, 자료 호스트는 Cheeky 커뮤니티 뷰어다. `Die` 트랙을 30Hz로 평가해 몸통·머리·양팔·양다리 등 15개 관절의 상대 회전과 몸 이동을 추출했다. 벨라는 부활용 `FakeDie`와 실제 `Die`를 구분하며, 게임의 부활 판정으로 살아남으면 사망 시퀀스를 시작하지 않는다. 클로에는 현재 일반 형태 모델에 대응하는 `Die`를 사용한다.

| 사도 | 참고 뷰어 | 원작 Die 길이 |
|---|---|---|
| 비비 | [vivi](https://wiki.cheeky.tsuncloud.com/korea/spines/vivi) | 2.000초 |
| 나이아 | [naia](https://wiki.cheeky.tsuncloud.com/korea/spines/naia) | 1.867초 |
| 에르핀 | [erpin](https://wiki.cheeky.tsuncloud.com/korea/spines/erpin) | 1.833초 |
| 이드 | [id](https://wiki.cheeky.tsuncloud.com/korea/spines/ed) | 2.567초 |
| 아야 | [aya](https://wiki.cheeky.tsuncloud.com/korea/spines/aya) | 2.500초 |
| 피코라 | [picora](https://wiki.cheeky.tsuncloud.com/korea/spines/picora) | 2.133초 |
| 클로에 | [chloe](https://wiki.cheeky.tsuncloud.com/korea/spines/chloe) | 1.833초 |
| 리뉴아 | [renua](https://wiki.cheeky.tsuncloud.com/korea/spines/renewaawaken) | 2.700초 |
| 마요 | [mayo](https://wiki.cheeky.tsuncloud.com/korea/spines/mayo) | 1.867초 |
| 벨라 | [bella](https://wiki.cheeky.tsuncloud.com/korea/spines/vela) | 3.000초 |
| 우이 | [ui](https://wiki.cheeky.tsuncloud.com/korea/spines/ui) | 1.833초 |
| 버터 | [butter](https://wiki.cheeky.tsuncloud.com/korea/spines/butter) | 1.867초 |
| 코미 | [komi](https://wiki.cheeky.tsuncloud.com/korea/spines/kommy) | 1.867초 |
| 힐데 | [hilde](https://wiki.cheeky.tsuncloud.com/korea/spines/hilde) | 1.867초 |
| 시온 더 다크불릿 | [shion](https://wiki.cheeky.tsuncloud.com/korea/spines/xxionx) | 1.867초 |

기존 모델에 맞춘 **3D 리타기팅**이다. 원작 2D 파츠 변형·표정·사라지는 효과까지 복제한 것은 아니다. 원작의 큰 화면 이탈 이동은 3D 장면에 맞게 압축하고 바닥 관통을 보정한다. 관절별 동작 차이와 시간은 사도별 표본에서 가져온다. 기존 Death 프레임 423–453을 정규화된 구간으로 사용하고 실제 재생 길이는 위 사도별 시간을 사용한다. 모델 형상·가중치·재질·다른 동작 키는 보존한다.

나이아·에르핀·리뉴아의 기존 Blend/FBX는 24fps였으나 런타임은 30fps 구간을 사용하고 있었다. 이 때문에 Death 시각이 실제로는 Dance 구간을 가리키던 문제를 함께 고쳤다. 세 파일의 프레임 기준을 30fps로 통일하고 공통 FBX 내보내기 함수에서 강제한다. 런타임은 전체 528프레임이 정확히 17.6초인 에셋만 허용한다.

## 구현과 재생성

- `src/deathcam.rs`: 최대 182프레임의 기록 버퍼, 탑뷰/리플레이/완료 상태, 보간, 실시간 월드와 분리된 표시용 월드.
- `src/motion.rs`: 일반 화면과 기록 화면이 공유하는 애니메이션 시계 및 사도별 사망 길이.
- `src/presentation/characters.rs`: 기록된 애니메이션 시각을 정확히 평가하고 사망 마지막 자세를 유지한다.
- `scripts/extract_death_motions.cjs`: 보관된 원작 파일에서 표본을 재추출한다. 연구용 Spine 런타임과 원본 리그는 게임에 로드하지 않는다.
- `scripts/bake_death_motions.py`: 기존 Blend/FBX의 Death 구간만 교체한다. 모델을 다른 제작 스크립트로 재생성한 경우에도 이 스크립트를 후처리로 실행한다.
- `assets/characters/death/motions.json`: 사도별 조사 표본. 변경 전 코드·모델, 출처 URL/해시, 표본 상세와 검증 결과는 `artifacts/death-replay/`에 보존한다.

네트워크 재생은 해당 클라이언트가 실제 받은 서버 스냅샷에 기반한다. 표본 사이 위치·조준·투사체는 보간하며 사망·공격·사도 변경 등 이산 사건은 기록 시점에 반영한다. 수신하지 못한 네트워크 구간의 사건을 새로 만들어 넣지 않는다.

## 검증

`cargo test --no-default-features`로 전투/UDP와 사망 시퀀스 회귀 검사를 실행한다. 사도별 전체 길이 대기, 녹화 전체 월드 되감기, 각도 경계 보간, 처치자 변경/연결 종료, 기록 부족, 부활·라운드·맵 전환을 검사한다. Blender 검사로 15리그/165클립/60루프와 관련 없는 키·메시의 보존을 확인한다.

`cargo run --bin tsb -- --deathcam-review`는 15명×4사망 자세, 녹화 처음/공격/처치 순간, 선택 화면, 부활, 다른 처치자로 전환 등 총 66장을 저장한다. 캡처 위치는 `artifacts/preview-death-replay-*-game.png`다.

최종 GPU 검사에서는 Blender의 Hips/Head 관절 위치 120개와 실제 엔진 관절 위치도 대조했다. 소수 프레임의 Bezier/FBX 보간 차이를 포함한 최대 오차는 약 4.7cm이며, 24/30fps 불일치로 동작 구간이 바뀌는 회귀를 검출한다. `--deathcam-review --death-hero 7`로 한 사도만 검사할 수도 있다.
