# 힐데 의료 총 방향 수정

힐데의 가져온 무기 메시에서 총구가 무기 기준 -Z, 실제 손잡이가 `(0, -0.06, 0.40)`에 있어 총구가 캐릭터를 향하고 손바닥도 손잡이에서 떨어져 있었다.

`src/presentation/weapons.rs`에서 힐데의 메시만 Y축으로 180도 돌리고 `(0, 0.06, 0.40)`만큼 옮겼다. 실제 손잡이는 기존 파지 원점에, 총구는 전방 +Z에 놓인다. 1인칭과 월드 무기는 같은 보정을 사용한다. `src/presentation.rs`의 발사 효과도 보정된 총구 위치에 맞췄다.

## 재현

```powershell
cargo build --locked --offline --release --bin tsb
target/release/tsb.exe --solo --auto-start --weapon-review --hilde-only --hud-review-small
target/release/tsb.exe --solo --auto-start --weapon-review --hilde-only --weapon-side-view --hud-review-small
```

`--hilde-only`는 1인칭 대기·공격·재장전과 월드 대기·공격을 검사한다. 기존 손가락·손바닥 접점 검사를 그대로 사용하며, 두 번째 명령은 월드 무기를 측면에서 촬영한다. 작업 자료와 독립된 검증 실행 폴더는 `artifacts/hilde-weapon-direction/`에 보관한다.

## 검증 결과

- Debug/Release 빌드, 변경한 Rust 파일의 포맷 검사, `cargo clippy --locked --offline --bin tsb -- -D warnings` 통과.
- 정면·측면 두 실행 모두 종료 코드 0 및 `HILDE_WEAPON_REVIEW_OK` 확인. 엔진 오류와 패닉 없음.
- 1인칭 대기·공격·재장전, 월드 정면·측면의 대기·공격 화면을 직접 확인해 총구가 전방을 향하는 것을 검증.
- `release/Play-HildeWeaponFix.cmd`로 검증한 실행 파일을 프로젝트 에셋과 함께 실행할 수 있다. 이 실행기는 현재 프로젝트 폴더에서 사용한다.
