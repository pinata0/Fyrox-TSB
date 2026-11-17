# 사망 모션 뒤 육면체 표시 수정

사망 모션이 끝난 뒤 이드의 녹화 시점으로 전환하면 화면 오른쪽에 큰 검은 육면체가 나타났다. 원인은 `weapons.rs`의 임시 `Quad laser frame`과 네 개의 `Laser emitter` 메시였다. 이드의 월드 모델에서는 숨겨져 있었지만 1인칭과 데스캠에서는 표시되고 있었다.

이드의 임시 손 무기 메시를 제거했다. 손 자세에 쓰는 루트와 접점, 발사 섬광 및 레이저 궤적은 유지한다. 사망 모션과 데스캠의 시간·카메라·부활 규칙은 변경하지 않는다.

로컬 수정판 실행: `release/Play-DeathCubeFix.cmd`를 더블클릭한다. 프로젝트 폴더에서 수정된 `target/release/tsb.exe`를 실행하므로 기존 에셋과 설정을 그대로 사용한다. 이 바로가기는 단독 배포용 파일이 아니다.

인증정보가 포함된 단일 EXE 재포장은 자동 승인 검사가 차단하여 생성하지 않았다.

## 검증

```powershell
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline --no-default-features deathcam
cargo build --locked --offline --release --bin tsb
target/release/tsb.exe --solo --auto-start --deathcam-review --death-transition-review --hud-review-small
```

전환 검증은 에르핀의 사망 네 자세, 이드 시점의 녹화 시작·발사·마지막 프레임, 선택창, 부활, 다른 처치자 전환까지 10개 화면을 캡처한다. 임시 무기 메시가 없고 녹화된 레이저가 표시되며 부활 후 자신의 무기가 복원되는지 검사한다. 결과와 전후 캡처는 `artifacts/death-cube-fix/`에 보관한다.

사용자가 사망한 캐릭터가 비비였다고 알려 준 뒤 검증 대상을 확대했다. `--death-transition-review`와 `--death-hero 0`을 함께 사용하면 비비의 전환을 검사한다. `--death-roster-review`는 현재 15명 모델의 사망 네 자세와 지정한 사망자의 데스캠·선택창·부활까지 66개 화면을 검사한다.

```powershell
cargo build --locked --offline --bin tsb
target/debug/tsb.exe --solo --auto-start --deathcam-review --death-roster-review --death-hero 0 --hud-review-small
```

비비의 마지막 사망 자세 및 비비가 사망한 뒤 이드 시점으로 넘어가는 장면에서 육면체가 나타나지 않는 것을 확인했다. 다른 14명의 마지막 사망 자세에서도 추가 육면체는 관찰되지 않았다. 이드의 임시 무기는 사망자 종류와 관계없이 처치자의 시점에 표시되므로 앞서 적용한 제거가 비비에게도 적용된다. 추가 검사 로그·66개 캡처·15명 마지막 자세 모음은 `artifacts/death-cube-fix/vivi-review/`에 있다. 이번 추가 작업은 검증 기능을 확장했으며, 앞서 제공한 로컬 release 실행판에도 동일한 무기 제거가 적용되어 있다.

기존 전체 `--deathcam-review`의 관절 좌표 기준은 이전 절차식 모델용이라 현재 모델의 첫 자세에서 실패한다. 새 `--death-transition-review`는 해당 기준과 독립적으로 전환·표시·재생 시간을 검사하며, 기존 전체 검사를 변경하거나 통과로 처리하지 않는다. 로컬 UDP 검사는 네트워크 제한이 없는 환경에서 실행해야 한다.
