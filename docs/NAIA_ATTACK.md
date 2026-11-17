# 나이아 땋은 머리와 일반 공격

2026-11-15. 현재 나이아 모델에 좌우 독립 머리카락 리그와 `Attack1_1` 전신·1인칭·표정을 적용했다.

- `Braid.L.01–03`, `Braid.R.01–03`의 6개 본을 추가했다. 각 체인은 Head 아래에 연결되며 두피 쪽 가중치를 완만하게 전환한다. 기존 메시 좌표, UV, 23개 얼굴 모프와 공격 외 신체 키는 유지한다.
- 보관된 원작 `naia-motion-reference.skel`을 Spine 런타임으로 평가했다. 좌우 `Hair_Tail_*`의 회전 곡선과 눈·입 파츠 전환 및 스케일을 `assets/characters/attack/naia-detail.json`에 저장한다.
- 원작 0.433초의 한쪽 웃는 눈, 0.800초의 웃는 입, 0.833초의 양눈 찡긋, 1.267초의 입 복귀, 1.300초의 눈 복귀를 같은 공격 진행률에 적용한다. 기존 3D 얼굴의 UV 위에 그린 표정이며 원작 스프라이트를 그대로 붙이는 방식은 아니다.
- 전신은 손 위치를 기준으로 팔꿈치를 계산한다. 원작 2D 팔의 360도 회전값 때문에 팔·물총이 잘못 뒤집히는 문제를 피하고, 준비 → 양손을 모아 전방으로 뻗기 → 반동 → 복귀 흐름을 적용한다. 1인칭 손과 물총도 같은 원작 경로를 사용한다.
- 원작 전체 길이는 1.667초이고, 게임에서는 기존 공격 간격에 맞춘 약 0.357초로 재생한다. 공격 속도 효과와 데스캠은 기존 공통 모션 시계를 사용한다. 피해량·판정 시점·공격 간격은 유지한다.

수정된 편집 원본은 `assets/characters/naia.blend`, 게임 모델은 `assets/characters/naia.fbx`다. 표정 재질과 얼굴 키도 Blender Attack 구간(156–180)에 들어 있다. 런타임 얼굴은 공격 진행률로 직접 평가해서 연사·중단·재생 위치 이동 때 이전 표정이 남지 않는다.

## 재생성 및 검사

이 작업의 원본 백업은 `artifacts/naia-attack/before/`에 있다. 리그 제작 스크립트는 이 백업을 기준으로 재생성하므로, 이후 별도로 수정한 나이아 원본이 있다면 먼저 백업 기준을 갱신한다.

```powershell
node scripts/extract_naia_attack.cjs
& 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe' --background --python-exit-code 1 --python scripts/rig_naia_braids.py
& 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe' --background --python-exit-code 1 --python scripts/bake_naia_attack_face.py
& 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe' --background --python-exit-code 1 --python scripts/export_meshy_morphs.py -- assets/characters naia
& 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe' --background --python-exit-code 1 --python scripts/verify_naia_attack.py
cargo run --bin tsb -- --solo --auto-start --attack-review --naia-only
```

원작 비교 이미지는 `artifacts/naia-attack/reference/source-contact.png`, Blender 앞·뒤·옆 렌더는 `artifacts/naia-attack/after/`에 저장한다. 검증은 좌우 본의 독립 변형, 정규화된 최대 4개 가중치, 101개 공격 표본의 유한 좌표 및 머리카락 과도 변형, 시작·끝 자세 복귀, FBX 본·모프 보존을 확인한다. 게임 검사는 전신·1인칭 총 12개 시점과 표정 시계, 무기 접촉을 확인한다.

자동 테스트 결과와 Clippy·빌드·GPU 로그는 `artifacts/naia-attack/`에 있다.

2026-11-15 최종 검증: 자동 테스트 201개, 포맷·Clippy·빌드 검사, Blender 공격 101개 표본 검사를 통과했다. 실제 게임에서도 전신 6개·1인칭 6개 시점을 캡처하고 `NAIA_ATTACK_REVIEW_OK`로 정상 종료했다. 표정 텍스처 4개와 58개 본, 23개 얼굴 모프의 내보내기도 확인했다. 요약 이미지는 `artifacts/naia-attack/naia-attack-preview.png`다.
