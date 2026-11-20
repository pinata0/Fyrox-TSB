# 피탄·베임 흔적과 파편

## 힐데 총구와 착탄 시간 보정 / 2026-11-20

힐데의 기본 주사기는 1인칭·월드 무기의 실제 총구에서 바늘 끝이 출발한다. 발사 순간의 총구를 한 번만 저장하고 0.1초 동안 조준 궤적으로 연결하므로, 이후 반동이나 이동에 이미 발사한 주사기가 끌려가지 않는다. 1인칭에서는 작은 무기에 맞는 크기로 출발해 같은 구간 동안 월드 크기로 돌아간다. 벽·바닥에 맞으면 파편과 착탄 자국이 보이며, 주사기와 자국은 4초 동안 남는다. 마지막 0.45초에 주사기를 축소해 제거한다.

착탄 효과 시계가 게임 틱보다 무한히 앞서던 문제를 수정했다. 정지한 스냅샷이나 느린 시뮬레이션 뒤에도 새 착탄이 즉시 만료되지 않도록 보간을 최대 0.1초로 제한한다. `--hilde-muzzle-review`는 실제 기본 공격 입력으로 1인칭·월드 총구와 바늘 끝의 일치를 검사하고 화면을 저장한다.

```powershell
cargo test --offline --bin tsb presentation::impacts
cargo test --offline --bin tsb presentation::projectiles::launch_tests
target/debug/tsb.exe --solo --auto-start --hilde-muzzle-review --lan --hud-review-small
target/debug/tsb.exe --solo --auto-start --impact-review --syringe-impacts-only --lan --hud-review-small
```

## 기본 탄환 외형과 잔류 탄환 / 2026-11-19

나이아는 파란 물폭탄, 버터는 조약돌, 에르핀은 발광 코어와 부드러운 광채가 있는 에너지볼, 마요는 초록 촉과 꼬리깃이 있는 독침, 힐데는 눈금·피스톤·바늘이 있는 주사기, 시온은 구리색 총알로 표시한다. 시온의 즉시 명중 판정은 유지하며 총알 외형만 기존 발사 경로를 따라 짧게 이동한다. 탄환 형상은 `src/presentation/projectiles.rs`에서 공유한다.

독침(강화 독침 포함)과 주사기는 구조물 착탄 기록으로 생성하여 기존 홈·파편과 함께 4초간 남긴다. 마지막 0.45초에 크기를 줄여 제거하며, 바늘 끝은 표면에 고정한다. 착탄 방향은 기존 패킷의 회전 바이트에 양자화해 저장하므로 32개 제한과 패킷 크기가 유지된다. 관전자·중도 참가·데스캠은 같은 종류·각도·원래 발생 시각을 사용한다. 캐릭터 피격과 공중 만료에는 잔류 객체를 만들지 않는다.

```powershell
cargo run --locked --bin tsb -- --solo --auto-start --projectile-review --hud-review-small
cargo run --locked --bin tsb -- --solo --auto-start --impact-review --hud-review-small
```

첫 명령은 여섯 탄환의 확대 GPU 화면을, 두 번째 명령은 기존 착탄 검사와 독침·주사기의 잔류·축소·소멸·바닥 착탄 화면을 저장한다.

즉시 명중하는 원거리 공격과 이동형 투사체가 맵 물체에 부딪히면 충돌면에 어두운 홈과 깨진 가장자리를 표시하고, 표면 색상을 따른 파편 7개를 바깥 방향으로 튀깁니다. 파편은 회전하며 중력으로 떨어지고 0.95초 안에 제거됩니다. 흔적은 4초 동안 유지되며 마지막 1.25초 동안 옅어집니다.

비비·아야·클로에·벨라·코미의 근접 공격은 끝이 가늘어지는 긴 베임 자국과 작은 파편 5개를 남깁니다. 표면 방향에 맞춰 휘두르는 궤적을 투영하며 연타 방향에 따라 기울기가 바뀝니다. 기본 자국 길이는 약 0.96m이며 거대화 시 함께 커집니다. 비비는 좌우 베기의 방향에 맞춰 자국의 기울기가 번갈아 바뀝니다. 베임 자국도 4초 뒤 사라집니다. 조준점과 근접 휘두르기 범위 안의 물체만 확인하므로 허공·사거리 밖·등 뒤에는 자국이 생기지 않습니다. 다단히트 공격도 한 번 휘두를 때 자국은 하나만 남깁니다.

실제 맵 재질의 색상과 법선을 일시적으로 바꾸는 시각 효과입니다. 지형 메시나 이동 충돌 형상은 바뀌지 않습니다. 빗나간 공격, 공중에서 만료된 투사체, 캐릭터에게 막힌 비관통 공격은 뒤쪽 벽에 흔적을 남기지 않습니다. 투사체의 충돌 반경을 보정해 흔적이 표면에서 떠 있지 않도록 합니다.

서버가 충돌 위치·방향·색상·크기·종류·회전·발생 틱을 확정합니다. 별도 `IMPACTS` 패킷은 최대 32개 흔적을 담아 기존 1,200바이트 제한을 지키며, 관전자와 중도 참가자도 원래 발생 시각을 받습니다. 동일 패킷을 다시 받아도 파편을 중복 생성하지 않습니다. 오래된 흔적부터 교체하고, 라운드·맵 변경 및 리플레이 되감기에서는 해당 시간대에 맞춰 정리합니다.

주요 구현은 `src/impact.rs`, `src/presentation/impacts.rs`, `src/presentation/impact_surface.glsl`입니다. 기존 유령 파문과 같은 장면 전용 재질을 공유하며 원본 FBX 재질은 수정하지 않습니다.

## 검증

```powershell
cargo test --locked --no-default-features --lib -- --test-threads=1
cargo test --locked --bin tsb presentation::impacts
cargo test --locked --bin tsb presentation::ghost_walls
cargo clippy --locked --all-targets -- -D warnings
cargo run --locked --bin tsb -- --solo --impact-review --auto-start
```

화면 검증은 실제 발사·근접 공격 입력으로 충돌을 만들고, 피탄 6개 화면과 베임·마무리 연타·파편·옅어짐·소멸·바닥 베임 6개 화면을 `artifacts/preview-impact-*-game.png`에 저장합니다. 로직 검증은 여섯 면의 법선과 베임 방향, 근접 캐릭터 5명, 투사체 접촉 위치, 빗나감과 만료, 개수 제한, 노드 제거, UDP 전송·중도 참가·오래된 패킷 무시를 확인합니다.
