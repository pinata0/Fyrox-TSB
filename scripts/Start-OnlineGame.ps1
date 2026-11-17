param(
    [Parameter(Mandatory = $true)][string]$SessionFile,
    [string]$GameExecutable,
    [string]$EosConfig,
    [switch]$ForceRelay
)
$ErrorActionPreference = 'Stop'
$sessionPath = (Resolve-Path -LiteralPath $SessionFile).Path
$gameRoot = Split-Path -Parent $PSScriptRoot
if (-not $GameExecutable) {
    $candidates = @((Join-Path $gameRoot 'tsb.exe'), (Join-Path $gameRoot 'target\release\tsb.exe'), (Join-Path $gameRoot 'target\debug\tsb.exe'))
    $GameExecutable = $candidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
}
if (-not $GameExecutable) { throw 'Build the game first: cargo build --release --bin tsb' }
$executable = (Resolve-Path -LiteralPath $GameExecutable).Path
Push-Location -LiteralPath $gameRoot
try {
    $gameArgs = @('--online-session', $sessionPath)
    if ($EosConfig) { $gameArgs += @('--eos-config', (Resolve-Path -LiteralPath $EosConfig).Path) }
    if ($ForceRelay) { $gameArgs += '--force-relay' }
    & $executable @gameArgs
    if ($LASTEXITCODE -ne 0) { throw 'Game startup failed; check the message above.' }
}
finally { Pop-Location }
