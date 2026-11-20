param([string[]]$Slugs = @('vivi','naia','erpin','id','aya','picora','chloe','renua','mayo','bella','ui','butter','komi','hilde','shion'))
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$blenderExe = 'C:\Program Files\Blender Foundation\Blender 5.3\blender.exe'
Push-Location $projectRoot
try {
    & $blenderExe --background --python-exit-code 1 --python scripts/publish_cartoon_hands.py -- @Slugs
    if ($LASTEXITCODE -ne 0) { throw 'Hand preparation failed; stopped before copying.' }
    foreach ($slug in $Slugs) {
        foreach ($suffix in @('.blend','.fbx','-hands.json','-rig.json','-cartoon-hands.png')) {
            Copy-Item -LiteralPath (Join-Path $projectRoot "artifacts/cartoon-hands/published/$slug$suffix") -Destination (Join-Path $projectRoot "assets/characters/$slug$suffix") -Force
        }
    }
    & $blenderExe --background --python-exit-code 1 --python scripts/publish_cartoon_hands.py -- --finalize
    if ($LASTEXITCODE -ne 0) { throw 'Hand manifest update failed.' }
    & $blenderExe --background --python-exit-code 1 --python scripts/export_meshy_morphs.py -- assets/characters @Slugs
    if ($LASTEXITCODE -ne 0) { throw 'Hand morph export failed.' }
} finally { Pop-Location }
