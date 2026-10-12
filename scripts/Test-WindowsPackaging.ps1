param([Parameter(Mandatory = $true)][string]$NsisCompiler)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$NsisCompiler = (Get-Command $NsisCompiler -CommandType Application -ErrorAction Stop).Source
$testRoot = Join-Path $root ('target/windows-packaging-qa-' + [Guid]::NewGuid().ToString('N'))
$workspace = Join-Path $testRoot 'workspace with spaces'
$utf8 = [Text.UTF8Encoding]::new($false)

function Write-Fixture([string]$RelativePath, [string]$Content) {
    $path = Join-Path $workspace $RelativePath
    New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force | Out-Null
    [IO.File]::WriteAllText($path, $Content, $utf8)
}

foreach ($relative in @(
    'scripts/Package-Windows.ps1', 'crates/yyplayer-app/build.rs',
    'assets/icons/yyplayer/liquid-orbit-disc/yyplayer.ico', 'packaging/windows/yyplayer.nsi',
    'packaging/windows/NSIS-BUILD-TOOL.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md',
    'docs/dependency-licenses.md', 'third_party/mpv/README.md',
    'config/cloud_api/example-gd.json', 'config/cloud_api/README.md'
)) {
    $destination = Join-Path $workspace $relative
    New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $root $relative) -Destination $destination
}
Write-Fixture 'Cargo.toml' @'
[workspace]
members = ["crates/yyplayer-app"]
resolver = "3"
[workspace.package]
version = "0.4.1"
'@
Write-Fixture 'crates/yyplayer-app/Cargo.toml' @'
[package]
name = "yyplayer-app"
version.workspace = true
edition = "2024"
[[bin]]
name = "yyplayer"
path = "src/main.rs"
'@
Write-Fixture 'crates/yyplayer-app/src/main.rs' 'fn main() {}'
# A tiny fixture DLL is only copied / hashed, never loaded or run.
Write-Fixture 'third_party/mpv/windows-x64/libmpv-2.dll' 'YYPlayer packaging fixture; not a runtime DLL.'
$fixtureHash = (Get-FileHash -LiteralPath (Join-Path $workspace 'third_party/mpv/windows-x64/libmpv-2.dll')).Hash.ToLowerInvariant()
$fixtureManifest = @{ library = 'libmpv-2.dll'; library_sha256 = $fixtureHash; build = 'packaging-fixture' } | ConvertTo-Json
Write-Fixture 'third_party/mpv/manifest.json' $fixtureManifest
Write-Fixture 'target/x86_64-pc-windows-msvc/release/yyplayer.exe' 'stale executable that must never be packaged'
$stalePath = Join-Path $workspace 'target/x86_64-pc-windows-msvc/release/yyplayer.exe'
$staleHash = (Get-FileHash -LiteralPath $stalePath).Hash
$packager = Join-Path $workspace 'scripts/Package-Windows.ps1'
$results = [Collections.Generic.List[string]]::new()
$previousEnvironment = @{}
foreach ($name in @('CARGO_TARGET_DIR', 'RUNNER_TEMP', 'GITHUB_SHA', 'YYPLAYER_PACKAGE_STAGE', 'YYPLAYER_PACKAGE_OUTPUT')) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name)
}

function Invoke-FixturePackage([string]$Case, [string]$ExpectedError = '') {
    $env:GITHUB_SHA = [Guid]::NewGuid().ToString('N')
    $before = @(Get-ChildItem -LiteralPath (Join-Path $workspace 'dist') -File -ErrorAction SilentlyContinue).Count
    $failure = $null
    try {
        & $packager -Offline -NsisCompiler $NsisCompiler *> (Join-Path $testRoot "$Case.log")
    } catch {
        $failure = $_.Exception.Message
    }
    if ($ExpectedError) {
        if (-not $failure -or $failure -notmatch $ExpectedError) { throw "$Case did not fail as expected: $failure" }
        $after = @(Get-ChildItem -LiteralPath (Join-Path $workspace 'dist') -File -ErrorAction SilentlyContinue).Count
        if ($before -ne $after) { throw "$Case published a package after failure." }
    } elseif ($failure) {
        throw "$Case failed: $failure"
    }
    if ((Get-FileHash -LiteralPath $stalePath).Hash -ne $staleHash) { throw 'The stale default artifact was changed.' }
    if (Get-ChildItem -LiteralPath $env:RUNNER_TEMP -Force) { throw "$Case left staging files behind." }
    if ($env:YYPLAYER_PACKAGE_STAGE -ne 'previous-stage' -or $env:YYPLAYER_PACKAGE_OUTPUT -ne 'previous-output') { throw "$Case did not restore the caller's environment." }
    $results.Add($Case)
}

Push-Location $testRoot
try {
    $env:CARGO_TARGET_DIR = Join-Path $workspace ('custom Cargo ' + [char]0x8f93 + [char]0x51fa)
    $env:RUNNER_TEMP = Join-Path $testRoot 'temp'
    New-Item -ItemType Directory -Path $env:RUNNER_TEMP | Out-Null
    $env:YYPLAYER_PACKAGE_STAGE = 'previous-stage'
    $env:YYPLAYER_PACKAGE_OUTPUT = 'previous-output'
    $ErrorActionPreference = 'Continue'
    & cargo generate-lockfile --offline --manifest-path (Join-Path $workspace 'Cargo.toml') *> (Join-Path $testRoot 'lockfile.log')
    $lockExitCode = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($lockExitCode -ne 0) { throw 'Could not generate the dependency-free fixture lockfile.' }

    Write-Fixture 'third_party/mpv/windows-x64/libmpv-2.dll' 'corrupt fixture'
    Invoke-FixturePackage 'runtime-hash-failure' 'pinned manifest hash'
    Write-Fixture 'third_party/mpv/windows-x64/libmpv-2.dll' 'YYPlayer packaging fixture; not a runtime DLL.'

    Write-Fixture 'crates/yyplayer-app/src/main.rs' 'this is intentionally invalid Rust'
    Invoke-FixturePackage 'build-failure-with-stale-exe' 'Cargo Release build failed'
    Write-Fixture 'crates/yyplayer-app/src/main.rs' 'fn main() {}'
    Invoke-FixturePackage 'custom-target-from-another-directory'

    $zip = Get-ChildItem -LiteralPath (Join-Path $workspace 'dist') -Filter '*.zip' | Select-Object -First 1
    $extracted = Join-Path $testRoot 'extracted'
    Expand-Archive -LiteralPath $zip.FullName -DestinationPath $extracted
    $builtExe = Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc/release/yyplayer.exe'
    $builtHash = (Get-FileHash -LiteralPath $builtExe).Hash
    if ((Get-FileHash -LiteralPath (Join-Path $extracted 'yyplayer.exe')).Hash -ne $builtHash) { throw 'ZIP contains the wrong executable.' }
    if ((Get-Item -LiteralPath (Join-Path $extracted 'yyplayer.exe')).VersionInfo.ProductVersion -ne '0.4.1') { throw 'ZIP has the wrong application version.' }
    $buildInfo = Get-Content -LiteralPath (Join-Path $extracted 'BUILD-INFO.txt') -Raw
    if ($buildInfo -notmatch $builtHash.ToLowerInvariant()) { throw 'BUILD-INFO does not identify the packaged executable.' }
    $results.Add('zip-artifact-hash-and-version')

    Write-Fixture 'packaging/windows/yyplayer.nsi' '!error "intentional packaging failure"'
    Invoke-FixturePackage 'nsis-failure-without-partial-package' 'NSIS failed'
    Copy-Item -LiteralPath (Join-Path $root 'packaging/windows/yyplayer.nsi') -Destination (Join-Path $workspace 'packaging/windows/yyplayer.nsi')
    Invoke-FixturePackage 'retry-after-failure-with-cached-artifact'

    # Keep the last SHA: a repeated request must preserve the existing packages.
    $beforeHashes = @(Get-ChildItem -LiteralPath (Join-Path $workspace 'dist') -File | Get-FileHash | Select-Object -ExpandProperty Hash)
    $failure = $null
    try { & $packager -Offline -NsisCompiler $NsisCompiler *> (Join-Path $testRoot 'existing-output.log') } catch { $failure = $_.Exception.Message }
    if ($failure -notmatch 'already exists') { throw 'Existing packages were not protected.' }
    $afterHashes = @(Get-ChildItem -LiteralPath (Join-Path $workspace 'dist') -File | Get-FileHash | Select-Object -ExpandProperty Hash)
    if (Compare-Object $beforeHashes $afterHashes) { throw 'Existing package contents changed.' }
    $results.Add('existing-packages-preserved')

    @{ result = 'PASS'; cases = $results.ToArray(); fixture = $testRoot; application_sha256 = $builtHash.ToLowerInvariant() } |
        ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $testRoot 'results.json') -Encoding UTF8
    Write-Output "PASS: $($results.Count) Windows packaging checks. Evidence: $testRoot"
} finally {
    Pop-Location
    foreach ($name in $previousEnvironment.Keys) {
        if ($null -eq $previousEnvironment[$name]) {
            Remove-Item -LiteralPath "Env:\$name" -ErrorAction SilentlyContinue
        } else {
            [Environment]::SetEnvironmentVariable($name, $previousEnvironment[$name])
        }
    }
}
