param([switch]$SkipBuild, [ValidateSet('debug', 'release')][string]$Profile = 'debug')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$qaRoot = Join-Path $projectRoot 'target/lyrics-ui'
$previousConfig = $env:YYPLAYER_CONFIG
Push-Location $projectRoot
try {
    & python (Join-Path $PSScriptRoot 'prepare-lyrics-fixtures.py')
    if ($LASTEXITCODE -ne 0) { throw 'Lyric fixture preparation failed' }
    if (-not $SkipBuild) {
        if ($Profile -eq 'release') {
            cargo build --locked --offline --release -p yyplayer-app --example lyrics-ui
        } else {
            cargo build --locked --offline -p yyplayer-app --example lyrics-ui --config profile.dev.debug=0 --config profile.dev.incremental=false
        }
        if ($LASTEXITCODE -ne 0) { throw 'Lyrics UI example build failed' }
    }
    $env:YYPLAYER_CONFIG = Join-Path $qaRoot 'settings.json'
    $qaStart = @{ FilePath = (Join-Path $projectRoot "target/$Profile/examples/lyrics-ui.exe"); WindowStyle = 'Hidden'; Wait = $true; PassThru = $true; RedirectStandardOutput = (Join-Path $qaRoot 'stdout.log'); RedirectStandardError = (Join-Path $qaRoot 'stderr.log') }
    if ($Profile -eq 'release') { $qaStart.ArgumentList = @('--release-motion') }
    $process = Start-Process @qaStart
    Get-Content -LiteralPath (Join-Path $qaRoot 'stdout.log')
    if ($process.ExitCode -ne 0) {
        Get-Content -LiteralPath (Join-Path $qaRoot 'stderr.log')
        throw "Lyrics UI qualification exited $($process.ExitCode)"
    }
} finally {
    $env:YYPLAYER_CONFIG = $previousConfig
    Pop-Location
}
