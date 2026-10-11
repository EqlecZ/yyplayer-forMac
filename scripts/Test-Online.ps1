param([switch]$SkipNavigation, [switch]$SkipHttps, [switch]$FrontendOnly)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$qaRoot = Join-Path $projectRoot 'target/online-qa'
$priorConfig = $env:YYPLAYER_CONFIG
$priorCa = $env:YYPLAYER_ONLINE_TEST_CA
$server = $null
Push-Location $projectRoot
try {
    & python (Join-Path $PSScriptRoot 'Serve-OnlineFixtures.py') --prepare
    if ($LASTEXITCODE -ne 0) { throw 'Fixture preparation failed (requires existing Python PIL/cryptography and FFmpeg).' }
    $server = Start-Process -FilePath (Get-Command python).Source -ArgumentList ('"' + (Join-Path $PSScriptRoot 'Serve-OnlineFixtures.py') + '"') -WorkingDirectory $projectRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $qaRoot 'server-output.log') -RedirectStandardError (Join-Path $qaRoot 'server-error.log')
    Start-Sleep -Milliseconds 700
    $server.Refresh()
    if ($server.HasExited) { throw 'Local fixture server failed; check target/online-qa/server-error.log (port 18443 must be free).' }
    $env:YYPLAYER_CONFIG = Join-Path $qaRoot 'ui-settings.json'
    $env:YYPLAYER_ONLINE_TEST_CA = Join-Path $qaRoot 'ca.pem'
    if (-not $SkipHttps) {
        & cargo test --locked --offline -p yyplayer-app --features online-qa --bin yyplayer https_end_to_end -- --ignored --nocapture
        if ($LASTEXITCODE -ne 0) { throw 'HTTPS contract/transfer regression failed.' }
    }
    $uiArguments = @()
    if ($FrontendOnly) { $uiArguments = @('--', '--frontend-only') }
    & cargo run --locked --offline -p yyplayer-app --features online-qa --example online-search @uiArguments
    if ($LASTEXITCODE -ne 0) { throw 'Actual Slint/controller/libmpv regression failed.' }
    if (-not $SkipNavigation) {
        & cargo run --locked --offline -p yyplayer-app --example navigation-ui
        if ($LASTEXITCODE -ne 0) { throw 'Actual sidebar navigation regression failed.' }
    }
    Write-Output ('PASS: Test-Online (self-owned HTTPS fixtures / independent settings / no OS trust changes; HTTPS contract skipped: ' + $SkipHttps + '; navigation skipped: ' + $SkipNavigation + '; frontend only: ' + $FrontendOnly + ').')
} finally {
    if ($server) { $server.Refresh(); if (-not $server.HasExited) { Stop-Process -Id $server.Id } }
    $env:YYPLAYER_CONFIG = $priorConfig
    $env:YYPLAYER_ONLINE_TEST_CA = $priorCa
    Pop-Location
}
