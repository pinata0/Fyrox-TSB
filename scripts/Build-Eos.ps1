param(
    [string]$SdkPath,
    [switch]$Release,
    [switch]$Check,
    [string]$ConfigFile,
    [switch]$ForceRelay
)
$ErrorActionPreference = 'Stop'
if ($ConfigFile -and -not $Check) { throw '-ConfigFile requires -Check' }
if ($ForceRelay -and -not $ConfigFile) { throw '-ForceRelay requires -Check -ConfigFile' }
$gameRoot = Split-Path -Parent $PSScriptRoot
$previousSdk = $env:EOS_SDK_DIR
try {
    if ($SdkPath) { $env:EOS_SDK_DIR = (Resolve-Path -LiteralPath $SdkPath).Path }
    if ($ConfigFile) { $ConfigFile = (Resolve-Path -LiteralPath $ConfigFile).Path }
    Push-Location -LiteralPath $gameRoot
    try {
        $cargoArgs = @('build', '--offline', '--features', 'eos', '--bin', 'tsb', '--bin', 'tsb-eos-check', '--bin', 'tsb-eos-peer-check')
        if ($Release) { $cargoArgs += '--release' }
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) { throw 'EOS build failed' }
        if ($Check) {
            $runArgs = @('run', '--offline', '--no-default-features', '--features', 'eos', '--bin', 'tsb-eos-check')
            if ($Release) { $runArgs += '--release' }
            $runArgs += '--'
            if ($ConfigFile) {
                $runArgs += @('--config', $ConfigFile)
                if ($ForceRelay) { $runArgs += '--force-relay' }
            } else { $runArgs += '--offline' }
            & cargo @runArgs
            if ($LASTEXITCODE -ne 0) { throw 'EOS check failed; see the diagnostic above' }
        }
    } finally { Pop-Location }
} finally { $env:EOS_SDK_DIR = $previousSdk }
