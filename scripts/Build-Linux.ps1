param(
    [string]$SdkPath,
    [ValidateRange(1, 32)][int]$Jobs = 4,
    [switch]$SkipImageBuild
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $SdkPath) {
    $candidates = @(Get-ChildItem (Join-Path (Split-Path $projectRoot -Parent) 'tools') -Directory |
        Where-Object { Test-Path (Join-Path $_.FullName 'SDK/Bin/libEOSSDK-Linux-Shipping.so') })
    if ($candidates.Count -ne 1) { throw 'Specify -SdkPath with the Linux EOS SDK directory.' }
    $SdkPath = Join-Path $candidates[0].FullName 'SDK'
}
$SdkPath = (Resolve-Path -LiteralPath $SdkPath).Path
if (Test-Path (Join-Path $SdkPath 'SDK/Include/eos_sdk.h')) { $SdkPath = Join-Path $SdkPath 'SDK' }
if (-not (Test-Path (Join-Path $SdkPath 'Bin/libEOSSDK-Linux-Shipping.so'))) {
    throw 'Linux x64 EOS runtime is missing from the selected SDK.'
}
Push-Location $projectRoot
try {
    if (-not $SkipImageBuild) {
        docker build -t tsb-linux-builder:ubuntu22.04 -f scripts/linux/Dockerfile scripts/linux
        if ($LASTEXITCODE -ne 0) { throw 'Linux builder image failed.' }
    }
    docker run --rm --name tsb-linux-build --cpus $Jobs `
        --mount "type=bind,source=$projectRoot,target=/workspace" `
        --mount "type=bind,source=$SdkPath,target=/eos-sdk,readonly" `
        --mount 'type=volume,source=tsb-linux-target,target=/linux-target' `
        --mount 'type=volume,source=tsb-linux-registry,target=/opt/cargo/registry' `
        -e EOS_SDK_DIR=/eos-sdk -e CARGO_TARGET_DIR=/linux-target -e "CARGO_BUILD_JOBS=$Jobs" `
        tsb-linux-builder:ubuntu22.04 bash /workspace/scripts/linux/build.sh
    if ($LASTEXITCODE -ne 0) { throw 'Linux build or verification failed. See artifacts/linux-build.' }
} finally { Pop-Location }
