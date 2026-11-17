param([switch]$Smoke)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $logDir = Join-Path $projectRoot 'artifacts/verification'
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
    $logPath = Join-Path $logDir ((Get-Date -Format 'yyyyMMdd-HHmmss') + '.log')
    function Invoke-Check([string[]]$CargoArguments) {
        ('cargo ' + ($CargoArguments -join ' ')) | Tee-Object -FilePath $logPath -Append
        & cargo @CargoArguments 2>&1 | Tee-Object -FilePath $logPath -Append
        if ($LASTEXITCODE -ne 0) { throw "Check failed: cargo $($CargoArguments -join ' ')" }
    }
    Invoke-Check @('fmt', '--all', '--', '--check')
    Invoke-Check @('test', '--locked', '--no-default-features')
    Invoke-Check @('clippy', '--locked', '--all-targets', '--', '-D', 'warnings')
    Invoke-Check @('build', '--locked', '--bins')
    if ($Smoke) { Invoke-Check @('run', '--locked', '--bin', 'tsb', '--', '--smoke-test') }
    'PASS: all requested checks completed.' | Tee-Object -FilePath $logPath -Append
    Write-Host "Verification log: $logPath"
} finally { Pop-Location }

