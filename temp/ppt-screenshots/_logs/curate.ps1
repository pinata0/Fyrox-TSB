$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$projectRoot = 'C:\Users\user\projects\Fyrox-TSB'
$outputDir = Join-Path $projectRoot 'temp\ppt-screenshots'
$shots = @(
    @('01_홈화면.png', 'preview-home-1920x1200-step0-menu.png', '표지·프로젝트 개요', '게임 로고와 홈 화면'),
    @('02_대전로비.png', 'preview-home-1920x1200-step20-menu.png', '플레이 흐름', '양 팀 슬롯·맵·선택형 봇을 보여주는 대전 로비'),
    @('03_맵선택.png', 'preview-home-1920x1200-step5-menu.png', '콘텐츠 소개', '요정왕국 성과 엘프 도시 선택'),
    @('04_캐릭터선택_에르핀.png', 'preview-cheeks-02-erpin-0-menu.png', '캐릭터·팀 편성', '15명 캐릭터 목록·팀 편성·에르핀 3D 모델. 06번의 볼 당기기 전 화면'),
    @('05_캐릭터선택_비비.png', 'preview-selection-00-vivi-0-menu.png', '캐릭터·아트', '비비 3D 모델과 툰 렌더링'),
    @('06_볼당기기_에르핀.png', 'preview-cheeks-02-erpin-2-menu.png', '원작 감성·상호작용', '볼 당기기 반응. 04번과 나란히 배치'),
    @('07_요정왕국성_왕좌실.png', 'preview-map-throne-game.png', '맵·배경 아트', 'HUD 없는 왕좌실 전경'),
    @('08_요정왕국성_거리.png', 'preview-map-street-game.png', '맵·배경 아트', 'HUD 없는 성 내부 거리'),
    @('09_요정왕국성_전체구조.png', 'preview-map-overview-game.png', '레벨 디자인', '전체 동선을 보여주는 조감도'),
    @('10_엘프도시_진입로.png', 'preview-elf-city-arrival-game.png', '맵 비교', 'HUD 없는 엘프 도시 야경'),
    @('11_엘프도시_전체구조.png', 'preview-elf-city-overview-game.png', '레벨 디자인', '도심 전장 조감도'),
    @('12_엘프도시_PC방.png', 'preview-elf-city-pc-cafe-game.png', '맵 세부 공간', 'HUD 없는 PC방 내부'),
    @('13_전투화면_아야.png', 'preview-game.png', '핵심 게임플레이', '11봇 연습 경기의 아야 1인칭·양산·궁극기 효과·전투 HUD'),
    @('14_일반스킬_마요.png', 'preview-mayo-skills-06-fp-Skill-dance-back.png', '캐릭터별 전투', '마요 일반 스킬의 소총과 1인칭 손'),
    @('15_궁극기_마요.png', 'preview-mayo-skills-09-fp-Ultimate-dance-back.png', '캐릭터별 전투', '마요 궁극기 무기와 1인칭 연출'),
    @('16_종족스킬_엘프포탈.png', 'preview-portal-05-game.png', '차별화 요소', '엘프 종족 공용 포탈의 실제 렌더링'),
    @('17_거점점령_HUD.png', 'preview-objective-1920x1200-b-half-game.png', '경기 규칙', 'A 확보 후 B 점령 진행 표시'),
    @('18_스코어보드.png', 'preview-scoreboard-1920x1200-full-game.png', '팀 전투·UI', '6대6 스코어보드와 개인 기록. 수치는 화면 검증용 데이터'),
    @('19_최종승리.png', 'preview-round-result-1920x1200-final-victory-game.png', '경기 흐름·마무리', '최종 승리 연출'),
    @('20_봇추가.png', 'preview-home-1920x1200-step18-menu.png', '혼자 연습·접근성', '원하는 캐릭터를 골라 봇을 추가하는 화면')
)
$rows = foreach ($shot in $shots) {
    $source = Join-Path $projectRoot ('artifacts\' + $shot[1])
    $destination = Join-Path $outputDir $shot[0]
    $sourceFile = Get-Item -LiteralPath $source
    if ($sourceFile.LastWriteTime -lt [datetime]'2026-11-21T06:13:00') { throw "Stale screenshot: $source" }
    Copy-Item -LiteralPath $source -Destination $destination
    $sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
    $destHash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
    if ($sourceHash -ne $destHash) { throw "Copy differs: $destination" }
    $bitmap = [System.Drawing.Image]::FromFile($destination)
    try {
        if ($bitmap.Width -ne 1920 -or $bitmap.Height -ne 1200) { throw "Unexpected resolution: $destination" }
        [pscustomobject]@{
            File = $shot[0]
            Slide = $shot[2]
            Description = $shot[3]
            Width = $bitmap.Width
            Height = $bitmap.Height
            CapturedAt = $sourceFile.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss')
            Source = 'artifacts/' + $shot[1]
            SHA256 = $destHash
        }
    } finally { $bitmap.Dispose() }
}
$rows | Export-Csv -LiteralPath (Join-Path $outputDir '스크린샷_목록.csv') -NoTypeInformation -Encoding utf8BOM
$notes = @'
Trickcal : Shooting Balddagu — 프로젝트 소개 PPT용 스크린샷

촬영일: 2026-11-21 (한국 시간)
해상도: 모든 개별 스크린샷 1920×1200, PNG, 16:10
촬영: 현재 target/release/tsb.exe의 게임 내 GPU 캡처 기능
01~20번: 새로 촬영한 원본을 그대로 복사. 창 테두리·다른 앱·마우스 커서 없음.
00_한눈에보기.png: 선택을 돕는 축소 미리보기. PPT에는 개별 원본 PNG를 사용하세요.

추천 배치
1. 표지 / 프로젝트 소개: 01
2. 캐릭터와 원작 감성: 04, 05, 06 (04와 06을 전후 비교로 사용)
3. 핵심 플레이와 거점 규칙: 13, 17
4. 캐릭터·종족별 스킬: 14, 15, 16
5. 두 전장 소개: 07 + 10, 구조 설명은 09 + 11
6. 로비와 혼자 연습: 02, 03, 20
7. 경기 결과 / 마무리: 18, 19
8. 배경 제작 세부 설명이 필요할 때: 08, 12

PPT 배치 팁
- 16:9 슬라이드에는 이미지 비율을 유지해 넣으세요. UI 화면을 꽉 채워 자르면 체력·스킬 표시가 잘릴 수 있습니다.
- 맵 전경에는 여백에 짧은 설명을, 기능 소개에는 한 장당 1~2개 이미지를 권장합니다.
- 장면별 설명과 원본 경로·해시는 스크린샷_목록.csv에 있습니다.

촬영 맥락
- 실제 게임 엔진 출력입니다. 기능 검증용으로 재현한 상태와 로컬 연습 경기를 촬영했습니다.
- 13번은 11봇 연습 경기입니다. 실제 다른 PC 플레이어와 온라인 대전한 사진은 아닙니다.
- 18번의 이름·점수·수치는 스코어보드 검증용 데이터입니다.
- 01번은 실제 홈 UI 캡처이며 배경은 게임에 들어 있는 일러스트입니다.

실행 기록
- 홈/로비, 두 맵, 15명 선택, 마요 스킬, 포탈, 경기 결과, 이름표, 스코어보드, 거점 HUD 촬영 루틴은 정상 종료했습니다.
- 볼 당기기 일괄 루틴은 이드의 변형 정점 수 검증에서 중단됐습니다. 선택한 에르핀 전후 화면은 그 전에 정상 저장됐고 직접 확인했습니다.
- 종합 스모크 루틴은 메뉴·전투·팀 화면을 저장했으나 최종 완료 조건에서 실패했습니다. 13번은 저장된 전투 화면이며, 전체 게임 테스트 통과를 의미하지 않습니다.
- 상세 실행 로그와 검토용 축소 이미지는 _logs 폴더에 보관했습니다.
'@
Set-Content -LiteralPath (Join-Path $outputDir 'PPT_활용안내.txt') -Value $notes -Encoding utf8BOM
$rows | Select-Object File,Width,Height,CapturedAt
