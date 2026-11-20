# 캐릭터 한국어 음성

[Trickcal Wiki 한국 서버 사도 도감](https://wiki.cheeky.tsuncloud.com/korea/apostles)의
공개 한국어 음성을 15명 전원에 적용했습니다. 기본 버전의 대사 255개에
E/Q 시전 음성 54개를 추가하여 총 309개를 사용합니다. 일본어 및 사복 음성은 섞지 않습니다.

**[누가 · 언제 · 무슨 대사를 말하는지 전체 목록](CHARACTER_VOICE_LINES.md)**에서
캐릭터별 대사 문장, 재생 상황, 원본 분류와 음원 링크를 볼 수 있습니다.

## 적용 위치

| 게임 상황 | 원본 분류 | 캐릭터당 수 |
| --- | --- | --- |
| 선택 화면 진입·다른 캐릭터 선택 | DeckSetting1/2 · 덱 편성 | 2 |
| 전장 진입·부활 | Spawn1/2 · 등장 | 2 |
| 경기 시작 | Stage · 스테이지 진입 | 1 |
| 승리·패배 | Victory1/2, Defeat1/2 | 4 |
| 사망 | Die1/2 · 퇴장 | 2 |
| 선택 화면에서 볼 잡기 | Touch1, Touch1_1, Touch1_2 · 볼 당기기 | 3 |
| C 누르기 → 음성 대사 → C 놓기 | Affinity1_1/2/3 · 친밀도 대사 | 3 |
| E 스킬 실제 발동 | SPSkill · 저학년 시전 | 2 |
| Q 궁극기 실제 발동 | Ultimate · 고학년 시전 | 1–2 |

같은 분류의 대사는 차례로 번갈아 재생합니다. 전투 음원의 이름과 문장은
[The Master's Journey](https://journey.927927927.xyz/)의 공개 한국어 대사 JSON에서
확인했고, 음원은 기존 Cheeky 아카이브에서 가져왔습니다. 각 문장의 출처 URL도
등록 정보에 저장합니다. 도감의 HeroSkill은 **스킬 강화 대사**이므로 사용하지 않습니다.
분할 녹음은 확인된 문장 단위 파일을 사용하며 문장 표기는 공개 뷰어의 표기를 따릅니다.

## 재생과 설정

- Esc → 설정에서 효과음 → 배경음악 → **캐릭터 음성 설정**으로 이동합니다.
  기본 75%, 0% 음소거이며 사용자 설정에 저장됩니다. 팀 마이크 음량과는 별개입니다.
  모든 캐릭터 대사에 재생 게인 0.5(약 -6dB)를 적용하여 기존보다 작게 재생합니다.
  저장된 음량 설정에도 적용되며 효과음·배경음악·팀 마이크 음량은 바꾸지 않습니다.
- 개인 선택·전투 반응은 본인에게 재생됩니다. C 메뉴 대사는 서버 승인 후
  주변 팀원에게도 거리·방향을 적용해 들립니다(최대 32m). 사용 간격은 4초입니다.
- E/Q 시전 대사는 실제 쿨다운 시작·궁극기 게이지 소모를 확인한 뒤 재생합니다.
  쿨다운 중 입력, 게이지 부족, 키를 계속 누르기와 동일 스냅샷에는 중복 재생하지 않습니다.
  주변 아군·적군의 시전도 발화자 슬롯과 위치를 구분해 최대 32m에서 들립니다.
  사망 리플레이 중에는 시전 대사를 재생하지 않습니다.
- 일반 채팅에 같은 문구를 입력해도 대사가 재생되지 않습니다. 서버가 승인한
  음성 표시만 처리하며, 재전송·과거 채팅·음소거 중 메시지는 다시 재생하지 않습니다.
- 한 사람은 한 번에 한 대사만 재생합니다. 승패 → 사망 → Q → E → 등장/시작 →
  선택/볼 당기기 → 일반 대사 순서로 우선합니다. 주변 인물 음성은 2개까지 재생합니다.
  설정 화면·포커스 이탈·음소거는 즉시 정지합니다.
- 음원은 EXE에 포함됩니다. 실행 시 인터넷이나 다운로드가 필요하지 않습니다.
  디코딩 캐시는 32개로 제한하며 대사 전체 길이와 원래 피치를 유지합니다.
- C 음성 표시는 팀 채팅 전달 패킷에 추가됩니다. 멀티플레이는 서버와 참가자가
  같은 업데이트 빌드를 사용해야 합니다.

## 파일과 출처

- `docs/CHARACTER_VOICE_LINES.md`: 15명 309개 대사의 발화자·상황·문장·음원 목록.
- `assets/audio/characters/<캐릭터>/`: 런타임용 모노 Vorbis, 44.1kHz, 약 10.10MB.
- `assets/audio/characters/sources.json`: 발화자·상황·문장·텍스트 출처·음원 URL·원본/변환본 SHA-256.
- `scripts/import_character_voices.py`: 공개 페이지에서 한국어 기본 음성을 선택하여 가져옵니다.
  원본 Ogg Opus는 `artifacts/character-voice-sources/raw/`에 보관합니다.
  Fyrox 호환을 위해 ffmpeg로 Vorbis 변환하며 잘라내거나 피치를 바꾸지 않습니다.
- `src/presentation/character_voice/bank.rs`: 가져오기 스크립트가 생성하는 내장 음원 목록.
- 실제 재생 로그 `CHARACTER_VOICE_PLAY`에는 실행 중 경과 초(`t`), 플레이어 슬롯,
  캐릭터 이름(`speaker`), 재생 상황(`cue`), 대사 문장(`text`)을 함께 남깁니다.
  등록 목록은 플레이 이력이 아니며 실제 재생 여부는 실행 로그에서 확인합니다.
- 원작 음성의 권리는 EPID Games 및 해당 권리자에게 있습니다. CC0 효과음과 별개이며,
  출처 목록이 재배포 허가를 부여하지는 않습니다.

## 확인

```powershell
python scripts/import_character_voices.py --offline
cargo test --lib --bin tsb --no-default-features --features desktop character_voice
cargo test --lib --bin tsb --no-default-features --features desktop sfx::
cargo run --bin tsb -- --solo --auto-start --character-voice-review
cargo run --bin tsb -- --solo --auto-start --settings-review --hud-review-small
```

음원 검사는 309개 전체 디코딩·길이 보존·정상 샘플·15명별 10종 분류를 검사합니다.
추가로 우선순위·중복/음소거 재생 방지·서버 표시와 일반 채팅의 분리·설정 마이그레이션을
검사합니다. `--character-voice-review`는 15명의 선택/E/Q 음성을 실제 장치에서 순서대로 재생하고
재생 시간 진행·음소거·포커스 처리를 확인한 뒤 자동 종료합니다.

2026-11-20 시전 대사 추가 검증:

- 309개 전체 디코딩·샘플 길이 보존·캐릭터별 10종 분류 검사 통과.
- 추가한 시전 음원 54개의 원본/변환본 길이 차이 최대 0.003598초.
- 캐릭터 음성·설정·서버 표시·효과음 관련 테스트 16개 통과.
- `cargo build --bin tsb` 기본 기능 빌드 통과.
- 실제 게임에서 15명 × 선택/E/Q = 45회 장치 재생, 음소거·포커스 검사 통과.
- 309행 목록과 등록 정보의 일치, 파일 해시, 오프라인 재생성 결과 일치 확인.

검증 기록: `artifacts/character-voice-review/casting-validation.json`.
실제 재생 문장 로그: `artifacts/character-voice-review/casting-playback.out`.
