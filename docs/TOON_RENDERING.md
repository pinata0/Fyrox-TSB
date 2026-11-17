# 트릭컬 화풍을 참고한 툰 렌더링

2026-11-16 적용. 기본 실행부터 캐릭터, 1인칭 팔·무기, 두 전장의 불투명 표면에 적용됩니다.

## 조사와 표현 방향

- [트릭컬 공식 사이트의 사복 세트·벨라 이미지](https://www.trickcal.com/revive/skinset/261112)
- [일본 공식 계정의 에르핀 캐릭터 소개](https://x.com/trickcal_jp/status/1928994910830788858)
- 프로젝트에 보관된 `assets/references/erpin-fullbody.png` 등 원작 캐릭터 참고 이미지

이미지에서 관찰한 특징은 둥근 실루엣, 분홍 볼과 큰 눈, 따뜻하고 밝은 색면, 짙은 갈색 계열 선,
넓고 단순한 그림자입니다. 이는 이미지에 대한 구현상의 해석이며, 원작의 렌더러 사양을 설명하는 것은 아닙니다.
기존 3D 모델의 비율·텍스처를 유지하고 광택과 명암, 윤곽선 표현을 이 방향으로 조정했습니다.

## 적용 내용

- 방향광·점광·스폿광의 PBR 응답을 **그림자 / 중간 / 밝음의 3단계 확산광**으로 교체합니다.
  색상 자체를 양자화하지 않으므로 눈·볼·의상에 그려진 색은 유지됩니다. 여러 광원이 겹치면 결과 명도는 합산됩니다.
- 따뜻한 환경광과 약한 재질 AO로 얼굴의 검은 음영을 줄입니다. 금속의 실시간 흰 반사광,
  SSAO와 블룸을 끄며, 기존 텍스처에 이미 그려진 하이라이트는 그대로 남습니다.
- 깊이와 노멀의 불연속을 이용해 갈색 윤곽선을 그립니다. 원근에 따른 평면의 깊이 변화는
  상쇄하고, 내부 선은 약하게, 먼 선은 옅게 처리합니다. 해상도에 맞춰 선 두께를 조정합니다.
- 윤곽선은 HDR 단계에서 합성하고 톤 매핑·FXAA를 거칩니다. 조준경의 주변 블러는 그 이후에
  적용되며 HUD·글자·초상화에는 윤곽선을 적용하지 않습니다.
- 메시 복제나 역방향 외피를 사용하지 않습니다. 원래 GBuffer와 알파 컷아웃, 스키닝, 볼·눈 변형,
  표정 텍스처 전환, 자기 몸의 그림자 전용 재질을 유지합니다. 유령 파문과 탄흔도 기존 재질에 남습니다.
- 물·보호막 등 별도의 투명 효과 재질은 기존 렌더링을 사용합니다.

## 코드와 실행

현재 워크스페이스에서 `artifacts/toon-rendering/Play-Toon.cmd`를 실행하면
프로젝트 루트를 작업 폴더로 삼아 이번에 빌드한 `target/debug/tsb.exe`를 엽니다.
기존 단일 EXE 배포판은 이번 작업에서 재패키징하지 않았습니다.

`src/presentation/toon.rs`가 Fyrox의 공개 렌더러 API로 조명 셰이더 4개와 윤곽선 패스 1개를 설치합니다.
맵이 바뀌면 대상 카메라·장면을 갱신하며, 그래픽 컨텍스트가 다시 만들어지면 재설치합니다.
셰이더는 `include_str!`로 실행 파일에 포함됩니다. Cargo 의존성이나 엔진 캐시는 수정하지 않습니다.

`src/presentation/toon/*light.shader`는 **Fyrox 1.0.1**의 원본 바인딩과 그림자 계산을 유지한 수정본이며
상단에 MIT 고지를 포함합니다. 엔진 버전을 올릴 때에는 uniform 레이아웃과 텍스처 바인딩을 함께 확인해야 합니다.
명암 경계·세기는 `lighting.glsl`, 선의 색·거리·경계값은 `outline.shader`에서 조절할 수 있습니다.

```powershell
cargo run --locked --offline --bin tsb
# 원래 PBR 렌더러와 비교하기 위한 진단 실행
cargo run --locked --offline --bin tsb -- --render-standard

# 화면 검증
target/debug/tsb.exe --solo --selection-review
target/debug/tsb.exe --solo --auto-start --map-review
target/debug/tsb.exe --solo --auto-start --map-review --map elf-city
target/debug/tsb.exe --solo --auto-start --weapon-review --shion-only --hud-review-small
```

## 검증

- `cargo fmt --all -- --check`, `cargo clippy --locked --offline --all-targets -- -D warnings`
- `cargo test --locked --offline --no-default-features`: **222개 통과**
- `cargo test --locked --offline --bin tsb`: **46개 통과, 기존 마이크 장치 테스트 1개 ignored**
- UDP가 필요한 기존 테스트는 제한된 샌드박스에서 패킷을 받지 못해 실패하여, 로컬 통신이 가능한 환경에서 재실행해 통과했습니다.
- 실제 OpenGL에서 조명 4개와 윤곽선 셰이더의 컴파일·링크 성공을 확인했습니다.
- 15명 선택 화면 30장, 요정왕국 성 19개 시점, 엘프 도시 11개 시점의 GPU 캡처를 확인했습니다.
- 960×640 창에서 시온의 일반 화면·조준·조준 중 명중·조준 해제·선택창 복귀 5개 GPU 검증을 통과했습니다.

로그와 캡처는 `artifacts/toon-rendering/`에 저장합니다.
`erpin-before-after.png`는 같은 선택 화면을 자른 전후 비교,
`roster-toon.png`는 15명 결과 모음입니다. 이는 실시간 엔진 출력이며 생성 이미지가 아닙니다.

이번 변경은 렌더링 표현에 한정합니다. 원작의 손그림 실루엣과 완전히 일치시키려면
모델 형태나 텍스처에 대한 별도 아트 작업이 필요합니다.
