param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $base = Join-Path $root 'target/navigation-history'
    New-Item -ItemType Directory -Path $base -Force | Out-Null
    if (-not $SkipBuild) {
        cargo build --locked --offline -p yyplayer-app --example navigation-history
        if ($LASTEXITCODE -ne 0) { throw 'Navigation history example build failed' }
    }
    # Actual production navigation and Slint callbacks in the preview controller,
    # without starting a media engine or reading/writing user settings.
    $process = Start-Process -FilePath (Join-Path $root 'target/debug/examples/navigation-history.exe') -WindowStyle Hidden -Wait -PassThru -RedirectStandardOutput (Join-Path $base 'stdout.log') -RedirectStandardError (Join-Path $base 'stderr.log')
    Get-Content -LiteralPath (Join-Path $base 'stdout.log')
    if ($process.ExitCode -ne 0) {
        Get-Content -LiteralPath (Join-Path $base 'stderr.log')
        throw "Navigation history qualification exited $($process.ExitCode)"
    }
} finally {
    Pop-Location
}
