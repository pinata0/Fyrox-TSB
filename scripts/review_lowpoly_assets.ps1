param([string[]]$Cases = @('blink', 'art', 'weapons', 'shadow', 'chloe', 'aya', 'ui', 'id'),
      [string]$Executable = './target/release/tsb.exe')
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
Set-Location $repo
$env:TSB_CHARACTER_ASSET_DIR = Join-Path $repo 'artifacts/lowpoly-import/characters'
$checks = @{
    blink = @('--selection-review', '--blink-review')
    art = @('--art-review')
    weapons = @('--weapon-review')
    shadow = @('--shadow-review')
    chloe = @('--chloe-skill-review')
    aya = @('--aya-skill-review')
    ui = @('--ui-skill-review')
    id = @('--id-skill-review')
}
foreach ($case in $Cases) {
    $arguments = @('--solo', '--auto-start', '--lan') + $checks[$case]
    & $Executable @arguments > "artifacts/lowpoly-qa-$case.log" 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Low-poly $case review failed: exit $LASTEXITCODE" }
    Write-Output "LOWPOLY_GPU_REVIEW_OK $case"
}
