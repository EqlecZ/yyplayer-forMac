param([Parameter(Mandatory = $true)][string]$NsisArchive)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$testRoot = Join-Path $root ('target/windows-launcher-qa-' + [Guid]::NewGuid().ToString('N'))
$workspace = Join-Path $testRoot ('workspace with spaces ' + [char]0x6587 + [char]0x4ef6 + ' (test)')
$utf8 = [Text.UTF8Encoding]::new($false)
$results = [Collections.Generic.List[string]]::new()
$legacyShell = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'

function Write-Fixture([string]$RelativePath, [string]$Content) {
    $path = Join-Path $workspace $RelativePath
    New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force | Out-Null
    [IO.File]::WriteAllText($path, $Content, $utf8)
}

foreach ($relative in @(
    'Package-Windows.cmd', 'scripts/Package-Windows.ps1', 'scripts/Package-Windows-Interactive.ps1',
    'scripts/Get-Nsis.ps1', 'scripts/Get-Mpv.ps1', 'packaging/windows/nsis.json',
    'crates/yyplayer-app/build.rs', 'assets/icons/yyplayer/liquid-orbit-disc/yyplayer.ico',
    'packaging/windows/yyplayer.nsi', 'packaging/windows/NSIS-BUILD-TOOL.md', 'LICENSE',
    'THIRD_PARTY_NOTICES.md', 'docs/dependency-licenses.md', 'third_party/mpv/README.md',
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
Write-Fixture 'third_party/mpv/windows-x64/libmpv-2.dll' 'Packaging fixture; never loaded.'
$library = Join-Path $workspace 'third_party/mpv/windows-x64/libmpv-2.dll'
$manifest = @{ library = 'libmpv-2.dll'; library_sha256 = (Get-FileHash -LiteralPath $library).Hash.ToLowerInvariant(); build = 'launcher-fixture' }
Write-Fixture 'third_party/mpv/manifest.json' ($manifest | ConvertTo-Json)
Compress-Archive -LiteralPath $library -DestinationPath (Join-Path $testRoot 'runtime-fixture.zip')
$manifest.archive_url = 'https://fixture.invalid/runtime.7z'
$manifest.archive_sha256 = (Get-FileHash -LiteralPath (Join-Path $testRoot 'runtime-fixture.zip')).Hash.ToLowerInvariant()
Write-Fixture 'third_party/mpv/manifest.json' ($manifest | ConvertTo-Json)

# Mock only the download; use the real installed extractor on an owned tiny archive.
function Invoke-RuntimeProbe([string]$Destination) {
    function Invoke-WebRequest {
        param([switch]$UseBasicParsing, [string]$Uri, [string]$OutFile)
        if ($Uri -ne 'https://fixture.invalid/runtime.7z') { throw 'Unexpected runtime fixture URL.' }
        Copy-Item -LiteralPath (Join-Path $testRoot 'runtime-fixture.zip') -Destination $OutFile
    }
    & (Join-Path $workspace 'scripts/Get-Mpv.ps1') -Destination $Destination
}
$runtimeProbe = Join-Path $testRoot 'runtime-probe'
Invoke-RuntimeProbe $runtimeProbe *> (Join-Path $testRoot 'runtime-extraction.log')
if ((Get-FileHash -LiteralPath (Join-Path $runtimeProbe 'libmpv-2.dll')).Hash.ToLowerInvariant() -ne $manifest.library_sha256) { throw 'Runtime extraction produced the wrong file.' }
$results.Add('verified-runtime-download-and-real-extraction')
$validArchiveHash = $manifest.archive_sha256
$manifest.archive_sha256 = '0' * 64
Write-Fixture 'third_party/mpv/manifest.json' ($manifest | ConvertTo-Json)
$runtimeRejected = $false
$badRuntimeProbe = Join-Path $testRoot 'bad-runtime-probe'
try { Invoke-RuntimeProbe $badRuntimeProbe *> (Join-Path $testRoot 'runtime-checksum-failure.log') }
catch { $runtimeRejected = $_.Exception.Message -match 'archive checksum mismatch' }
if (-not $runtimeRejected -or (Test-Path -LiteralPath (Join-Path $badRuntimeProbe 'libmpv-2.dll'))) { throw 'Runtime archive with a wrong hash was extracted.' }
$manifest.archive_sha256 = $validArchiveHash
Write-Fixture 'third_party/mpv/manifest.json' ($manifest | ConvertTo-Json)
$results.Add('runtime-archive-checksum-before-extraction')
Write-Fixture 'pause-input.txt' "x`r`n"
$archive = Join-Path $workspace 'target/windows-packaging-tools/nsis-3.12.zip'
$launcher = Join-Path $workspace 'scripts/Package-Windows-Interactive.ps1'
$cmdLauncher = Join-Path $workspace 'Package-Windows.cmd'

function Get-Packages { @(Get-ChildItem -LiteralPath (Join-Path $workspace 'dist') -File -Recurse -ErrorAction SilentlyContinue) }

function Invoke-Launcher([string]$Case, [string]$ExpectedError = '', [switch]$Cmd) {
    $before = @(Get-Packages | Get-FileHash | Select-Object -ExpandProperty Hash)
    $log = Join-Path $testRoot "$Case.log"
    $ErrorActionPreference = 'Continue'
    if ($Cmd) {
        # The same .cmd Explorer launches, with one key supplied to its final pause.
        $arguments = '/d /c ""' + $cmdLauncher + '" -Offline -NoOpen < "' + (Join-Path $workspace 'pause-input.txt') + '""'
        $process = Start-Process -FilePath $env:ComSpec -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru -RedirectStandardOutput $log -RedirectStandardError ($log + '.stderr')
        $nativeExitCode = $process.ExitCode
        Get-Content -LiteralPath ($log + '.stderr') | Add-Content -LiteralPath $log
    } else {
        & $legacyShell -NoLogo -NoProfile -ExecutionPolicy Bypass -File $launcher -Offline -NoOpen *> $log
    }
    $exitCode = if ($Cmd) { $nativeExitCode } else { $LASTEXITCODE }
    $ErrorActionPreference = 'Stop'
    $after = @(Get-Packages | Get-FileHash | Select-Object -ExpandProperty Hash)
    $output = Get-Content -LiteralPath $log -Raw
    if ($ExpectedError) {
        if ($exitCode -eq 0 -or $output -notmatch $ExpectedError) { throw "$Case returned $exitCode without the expected error; see $log" }
        if (Compare-Object $before $after) { throw "$Case changed package outputs after failure." }
    } else {
        if ($exitCode -ne 0 -or $after.Count -ne $before.Count + 2) { throw "$Case failed to publish both packages; see $log" }
        foreach ($hash in $before) { if ($after -notcontains $hash) { throw "$Case overwrote a previous package." } }
    }
    $latestRun = Get-ChildItem -LiteralPath (Join-Path $workspace 'target/windows-packaging-runs') -Directory | Sort-Object LastWriteTime | Select-Object -Last 1
    $transcript = Join-Path $latestRun.FullName 'packaging.log'
    if (-not (Test-Path -LiteralPath $transcript) -or (Get-Item -LiteralPath $transcript).Length -eq 0) { throw "$Case did not retain its log." }
    $results.Add($Case)
}

$previousEnvironment = @{}
foreach ($name in @('CARGO_TARGET_DIR', 'GITHUB_SHA', 'RUNNER_TEMP')) { $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name) }
Push-Location $testRoot
try {
    $env:CARGO_TARGET_DIR = Join-Path $workspace 'custom Cargo output'
    $env:GITHUB_SHA = '0123456789abcdef0123456789abcdef01234567'
    Remove-Item -LiteralPath Env:RUNNER_TEMP -ErrorAction SilentlyContinue
    $ErrorActionPreference = 'Continue'
    & cargo.exe generate-lockfile --offline --manifest-path (Join-Path $workspace 'Cargo.toml') *> (Join-Path $testRoot 'lockfile.log')
    $lockExitCode = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($lockExitCode -ne 0) { throw 'Cannot create fixture lockfile.' }

    Invoke-Launcher 'missing-tool-offline' 'NSIS archive is missing or corrupt'
    Write-Fixture 'target/windows-packaging-tools/nsis-3.12.zip' 'corrupt zip'
    Invoke-Launcher 'corrupt-tool-offline' 'NSIS archive is missing or corrupt'
    Copy-Item -LiteralPath $NsisArchive -Destination $archive -Force
    Invoke-Launcher 'powershell51-success-outside-cwd'
    Invoke-Launcher 'cmd-repeat-same-version-and-commit' -Cmd

    $packages = @(Get-Packages)
    $parents = @($packages.DirectoryName | Sort-Object -Unique)
    if ($parents.Count -ne 2) { throw 'Repeated launches did not use independent output directories.' }
    $zip = $packages | Where-Object Extension -eq '.zip' | Select-Object -First 1
    $unpacked = Join-Path $testRoot 'unpacked'
    Expand-Archive -LiteralPath $zip.FullName -DestinationPath $unpacked
    $exe = Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc/release/yyplayer.exe'
    if ((Get-FileHash -LiteralPath (Join-Path $unpacked 'yyplayer.exe')).Hash -ne (Get-FileHash -LiteralPath $exe).Hash) { throw 'Launcher packaged the wrong executable.' }
    if ((Get-Item -LiteralPath (Join-Path $unpacked 'yyplayer.exe')).VersionInfo.ProductVersion -ne '0.4.1') { throw 'Launcher packaged the wrong version.' }
    $results.Add('actual-artifact-version-and-hash')

    Write-Fixture 'crates/yyplayer-app/src/main.rs' 'deliberately invalid Rust'
    Invoke-Launcher 'compile-failure-does-not-package-old-exe' 'Cargo Release build failed' -Cmd
    Write-Fixture 'crates/yyplayer-app/src/main.rs' 'fn main() {}'
    Write-Fixture 'third_party/mpv/windows-x64/libmpv-2.dll' 'corrupt runtime'
    Invoke-Launcher 'runtime-hash-failure' 'pinned hash'
    Remove-Item -LiteralPath $library
    Invoke-Launcher 'missing-runtime-offline' 'runtime is missing'

    @{ result = 'PASS'; cases = $results.ToArray(); fixture = $testRoot; output_directories = $parents } |
        ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $testRoot 'results.json') -Encoding UTF8
    Write-Output "PASS: $($results.Count) Windows launcher checks. Evidence: $testRoot"
} finally {
    Pop-Location
    foreach ($name in $previousEnvironment.Keys) {
        if ($null -eq $previousEnvironment[$name]) { Remove-Item -LiteralPath "Env:\$name" -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($name, $previousEnvironment[$name]) }
    }
}
