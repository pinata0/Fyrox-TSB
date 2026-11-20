$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
Set-Location $repo
python scripts/publish_lowpoly_assets.py --prepare
if ($LASTEXITCODE -ne 0) { throw 'Low-poly publication preparation failed' }
$assetRoot = [IO.Path]::GetFullPath((Join-Path $repo 'assets/characters')) + [IO.Path]::DirectorySeparatorChar
$pending = Get-Content -LiteralPath 'artifacts/lowpoly-import/publish-pending.json' -Raw | ConvertFrom-Json
foreach ($entry in $pending) {
    foreach ($path in @($entry.source, $entry.target)) {
        if (-not [IO.Path]::GetFullPath($path).StartsWith($assetRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Publication path is outside the character asset directory: $path"
        }
    }
}
foreach ($entry in $pending) {
    Move-Item -LiteralPath $entry.source -Destination $entry.target -Force
}
python scripts/publish_lowpoly_assets.py --verify
if ($LASTEXITCODE -ne 0) { throw 'Low-poly published asset verification failed' }
