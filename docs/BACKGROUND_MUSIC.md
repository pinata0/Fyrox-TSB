# 배경음악

| 화면 | 곡 |
| --- | --- |
| 메인 화면 / 방 목록 | Trickal Main Title |
| 전투 로비 | Day Life |
| 영웅 선택 | Lounge |
| 인게임 / 전투 결과 | Adventure Begins |

네 곡 모두 Warak의 Trickcal Revive Soundtrack Collection에 있는 실제 OST다.
[수록곡 및 다운로드 출처](https://downloads.khinsider.com/game-soundtracks/album/trickcal-revive-soundtrack-collection-2024),
[작곡가의 OST 모음](https://www.youtube.com/watch?v=8kKIzim7ZqU).
각 파일 출처와 SHA-256은 `assets/audio/music/sources.json`에 기록했다.
효과음의 Kenney CC0 라이선스와 별개이며 원곡 권리는 원권리자에게 있다.

전체 길이의 스테레오 Ogg Vorbis를 실행 파일에 포함하므로 네트워크 없이 재생된다.
FLAC 원본에서 음량을 -18 LUFS로 맞췄으며 잘라내거나 곡을 합치지 않았다.
`scripts/import_music.py`와 ffmpeg로 다시 가져올 수 있다.
재생은 스트리밍 버퍼를 사용해 네 곡 전체를 PCM으로 메모리에 올리지 않는다.

한 번에 한 곡만 재생한다. 화면 전환 시 0.25초 동안 기존 곡을 줄이고 새 곡을
0.4초 동안 키운다. 각 곡은 끝나면 처음부터 반복된다. 설정, 채팅, 도움말, 점수판은
곡을 바꾸지 않는다. 맵 교체 시 재생 위치를 유지하며 창이 비활성화되면 일시 정지한다.

기본 배경음악 음량은 35%다. **Esc → 설정 → 효과음 설정 → 배경음악 설정**에서
별도로 조절하고, 0%로 음소거할 수 있다. 변경값은 사용자 설정에 저장한다.

검증 명령:

```text
cargo test --lib --no-default-features settings::
cargo test --bin tsb --no-default-features --features desktop presentation::music::
cargo build --bin tsb
target/debug/tsb.exe --solo --auto-start --music-review
target/debug/tsb.exe --solo --auto-start --settings-review
```

2026-11-20 검증: 전체 파일 디코딩, 화면 우선순위, 중복 소스 방지, 같은 곡 유지,
포커스/음소거, 맵 교체 시 재생 위치를 다루는 음악 테스트 4개와 설정 테스트 4개 통과.
기본 기능을 포함한 `tsb` 빌드 성공. 실제 오디오 장치에서 네 곡의 재생 위치가
전진하고, 곡 끝에서 처음으로 반복하며, 음소거·포커스 복귀가 작동하는 것까지
`artifacts/music-review/playback.log`의 `MUSIC_REVIEW_OK`로 확인했다.
설정 화면도 실제 실행에서 20%, 음소거, 기본값 복원, 효과음과의 독립 저장을 검증했다.
결과는 `artifacts/music-review/settings.log`의 `SETTINGS_REVIEW_OK`와
`artifacts/preview-settings-16-game.png`부터 `19-game.png`까지의 화면 캡처로 남겼다.
