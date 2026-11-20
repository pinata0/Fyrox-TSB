$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
Remove-Item Env:TSB_CHARACTER_ASSET_DIR -ErrorAction SilentlyContinue
Remove-Item Env:TSB_PERF_VARIANT -ErrorAction SilentlyContinue
$env:TSB_PERF_SECONDS = '42'
$env:TSB_PERF_NO_VSYNC = '1'
$cases = @(
    @{ Name = 'idle'; Extra = @() },
    @{ Name = 'bots'; Extra = @('--perf-bots') },
    @{ Name = 'city'; Extra = @('--map', 'elf-city') }
)
foreach ($case in $cases) {
    $arguments = @('--solo', '--auto-start', '--lan', '--perf-profile') + $case.Extra
    & './target/release/tsb.exe' @arguments > "artifacts/perf-lowpoly-$($case.Name).log" 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Benchmark $($case.Name) failed: $LASTEXITCODE" }
    Write-Output "LOWPOLY_BENCHMARK_OK $($case.Name)"
}
python artifacts/compare_perf.py release-before verified-idle verified-bots verified-city lowpoly-idle lowpoly-bots lowpoly-city
