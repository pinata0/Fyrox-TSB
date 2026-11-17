# 2CP 경기 HUD

전투 집중형 시안에서 팀 점수·시간과 A/B 거점 표시만 적용했습니다.

- 상단: BLOOM / COCOA 이름, 팀별 거점 점수, 공격·수비 역할, 라운드, 남은 공격 시간.
- 하단: A/B 마름모. 현재 거점은 공격팀 색으로 아래부터 채우며 채운 **면적**이 실제 점령률과 일치합니다.
- A 확보 후에는 A를 가득 채우고 B를 갱신합니다. 공수 교대 때 공격팀 색과 거점 상태를 새 라운드에 맞춥니다.
- 격돌은 현재 마름모 테두리를 금색으로 표시합니다. 추가시간은 타이머 색과 짧은 타이머 라벨로 표시합니다.
- 상대 팀이 없는 자유 연습은 `∞`로 표시합니다.
- 점령 상태 문장, 별도 점령률 숫자·막대, 모드 이름·예시 설명은 표시하지 않습니다.
- 선택창·로비·홈·라운드 결과 화면에서는 숨깁니다. 데스캠 중 점수·시간·점령률은 현재 서버 상태를 표시합니다.

구현: `src/presentation/objective_hud.rs`. 판정·네트워크 규칙은 변경하지 않습니다.

## 검증

```powershell
cargo test --locked --offline --bin tsb objective_hud
pwsh -File scripts/verify.ps1
target/debug/tsb.exe --solo --auto-start --objective-hud-review
target/debug/tsb.exe --solo --auto-start --objective-hud-review --hud-review-small
```

GPU 검증은 A의 0/25/50/75%, A 완료+B 진행, COCOA 공격, 추가시간·격돌, 자유 연습, 선택창 숨김의 9개 상태를 캡처합니다. 결과는 `artifacts/preview-objective-*`에 저장됩니다.
