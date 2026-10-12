param(
    [switch]$Offline,
    [string]$NsisCompiler = ''
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$runtimeRoot = Join-Path $projectRoot 'third_party/mpv/windows-x64'
$appIconPath = Join-Path $projectRoot 'assets/icons/yyplayer/liquid-orbit-disc/yyplayer.ico'
$target = 'x86_64-pc-windows-msvc'
$distRoot = Join-Path $projectRoot 'dist'
$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$tempRoot = [IO.Path]::GetFullPath($tempRoot)
$workRoot = Join-Path $tempRoot ('yyplayer-windows-x64-' + [Guid]::NewGuid().ToString('N'))
$portableStage = Join-Path $workRoot 'portable'

if ($env:OS -ne 'Windows_NT') { throw 'Windows x64 packaging requires Windows and the MSVC toolchain.' }
if (-not (Test-Path -LiteralPath $appIconPath -PathType Leaf)) { throw "Windows application icon not found: $appIconPath" }
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'third_party/mpv/manifest.json') -Raw | ConvertFrom-Json
$library = Join-Path $runtimeRoot $manifest.library
if (-not (Test-Path -LiteralPath $library -PathType Leaf)) { throw 'The verified mpv runtime has not been extracted. Run scripts/Get-Mpv.ps1 first.' }
$libraryHash = (Get-FileHash -LiteralPath $library -Algorithm SHA256).Hash.ToLowerInvariant()
if ($libraryHash -ne $manifest.library_sha256) { throw 'Extracted libmpv DLL does not match the pinned manifest hash.' }

if ($NsisCompiler) {
    $compiler = (Get-Command $NsisCompiler -CommandType Application -ErrorAction Stop).Source
} else {
    $nsisCommand = Get-Command makensis.exe -CommandType Application -ErrorAction SilentlyContinue
    $compiler = if ($nsisCommand) { $nsisCommand.Source } else { $null }
    foreach ($installRoot in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if (-not $compiler -and $installRoot) {
            $candidate = Join-Path $installRoot 'NSIS/makensis.exe'
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { $compiler = $candidate }
        }
    }
}
if (-not $compiler) { throw 'NSIS is required. Put makensis.exe on PATH or pass -NsisCompiler with its full path.' }
$nsisVersion = & $compiler /VERSION
if ($LASTEXITCODE -ne 0) { throw 'Could not query the NSIS compiler version.' }
$nsisVersion = ($nsisVersion -join '').Trim()
$cargo = (Get-Command cargo.exe -CommandType Application -ErrorAction Stop).Source

# Cargo metadata is the source of the inherited application version and package ID.
# Running from the repository also applies its Cargo config regardless of the caller's cwd.
Push-Location $projectRoot
try {
    $metadataArgs = @('metadata', '--locked', '--no-deps', '--format-version', '1')
    if ($Offline) { $metadataArgs += '--offline' }
    # Windows PowerShell 5.1 can turn native stderr progress into an error record.
    # Check the exit code ourselves; progress must not abort a successful build.
    $ErrorActionPreference = 'Continue'
    $metadataJson = & $cargo @metadataArgs
    $cargoExitCode = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($cargoExitCode -ne 0) { throw "Cargo metadata failed with exit code $cargoExitCode." }
    $metadata = ($metadataJson -join "`n") | ConvertFrom-Json
} finally {
    $ErrorActionPreference = 'Stop'
    Pop-Location
}
$appPackages = @($metadata.packages | Where-Object { $_.name -eq 'yyplayer-app' })
if ($appPackages.Count -ne 1) { throw 'Cargo metadata must contain exactly one yyplayer-app package.' }
$appPackage = $appPackages[0]
$version = $appPackage.version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Windows packages require a three-part application version.' }
$commit = $env:GITHUB_SHA
if (-not $commit) {
    $commit = git -C $projectRoot rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Could not read the source commit.' }
    $commit = ($commit -join '').Trim()
}
$dirty = git -C $projectRoot status --porcelain
if ($LASTEXITCODE -ne 0) { throw 'Could not read the source working tree status.' }
$shortCommit = $commit.Substring(0, [Math]::Min(12, $commit.Length))
$portableZip = Join-Path $distRoot "YYPlayer-$version-$shortCommit-windows-x64-portable.zip"
$setupExe = Join-Path $distRoot "YYPlayer-$version-$shortCommit-windows-x64-setup.exe"
if ((Test-Path -LiteralPath $portableZip) -or (Test-Path -LiteralPath $setupExe)) { throw 'A package for this commit already exists in dist.' }

# Always build, and only use the executable reported by this successful Cargo invocation.
# This also supports CARGO_TARGET_DIR / build.target-dir without guessing a path.
$artifacts = @()
$buildSucceeded = $false
Push-Location $projectRoot
try {
    $buildArgs = @('build', '--locked', '--release', '--target', $target, '-p', 'yyplayer-app', '--bin', 'yyplayer', '--message-format=json')
    if ($Offline) { $buildArgs += '--offline' }
    Write-Host "Building YYPlayer $version for $target..."
    $ErrorActionPreference = 'Continue'
    & $cargo @buildArgs | ForEach-Object {
        if ($_.StartsWith('{')) {
            $message = $_ | ConvertFrom-Json -ErrorAction Stop
            if ($message.reason -eq 'compiler-message' -and $message.message.rendered) {
                Write-Host $message.message.rendered
            } elseif ($message.reason -eq 'compiler-artifact' -and $message.package_id -eq $appPackage.id -and
                $message.target.name -eq 'yyplayer' -and $message.target.kind -contains 'bin' -and
                -not $message.profile.test -and $message.executable) {
                $artifacts += $message.executable
            } elseif ($message.reason -eq 'build-finished') {
                $buildSucceeded = $message.success
            }
        } else {
            Write-Host $_
        }
    }
    $cargoExitCode = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($cargoExitCode -ne 0) { throw "Cargo Release build failed with exit code $cargoExitCode; no package was created." }
} finally {
    $ErrorActionPreference = 'Stop'
    Pop-Location
}
if (-not $buildSucceeded -or $artifacts.Count -ne 1) { throw 'Cargo did not report one successfully built yyplayer executable.' }
$exePath = $artifacts[0]
if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) { throw "Built executable not found: $exePath" }
if ((Get-Item -LiteralPath $exePath).VersionInfo.ProductVersion -ne $version) { throw 'Built executable version does not match the Cargo application version.' }
$exeHash = (Get-FileHash -LiteralPath $exePath -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "Packaging Cargo artifact: $exePath"

$previousStage = $env:YYPLAYER_PACKAGE_STAGE
$previousOutput = $env:YYPLAYER_PACKAGE_OUTPUT
New-Item -ItemType Directory -Path $workRoot | Out-Null
try {
    New-Item -ItemType Directory -Path $portableStage | Out-Null
    Copy-Item -LiteralPath $exePath -Destination (Join-Path $portableStage 'yyplayer.exe')
    if ((Get-FileHash -LiteralPath (Join-Path $portableStage 'yyplayer.exe') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $exeHash) { throw 'Staged executable does not match the Cargo artifact.' }
    $runtimeStage = Join-Path $portableStage 'runtime'
    New-Item -ItemType Directory -Path $runtimeStage | Out-Null

    # Keep the Windows DLL dependency closure and runtime data. Drop development
    # headers, import/static libraries, debug files, and standalone player binaries.
    $excludedExtensions = @('.h', '.hpp', '.lib', '.a', '.pdb', '.7z', '.exe', '.com')
    foreach ($entry in Get-ChildItem -LiteralPath $runtimeRoot -Force) {
        if ($entry.PSIsContainer) {
            if ($entry.Name -ne 'include') {
                Copy-Item -LiteralPath $entry.FullName -Destination $runtimeStage -Recurse
            }
        } elseif ($excludedExtensions -notcontains $entry.Extension.ToLowerInvariant()) {
            Copy-Item -LiteralPath $entry.FullName -Destination $runtimeStage
        }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $runtimeStage $manifest.library) -PathType Leaf)) { throw 'Packaged libmpv DLL is missing.' }
    if ((Get-FileHash -LiteralPath (Join-Path $runtimeStage $manifest.library) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.library_sha256) { throw 'Packaged libmpv DLL does not match the pinned manifest hash.' }
    if (-not (Get-ChildItem -LiteralPath $runtimeStage -Filter '*.dll' -File -Recurse)) { throw 'No runtime DLLs were copied into the package.' }

    $cloudApiStage = Join-Path $portableStage 'config/cloud_api'
    New-Item -ItemType Directory -Path $cloudApiStage -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $projectRoot 'config/cloud_api/example-gd.json') -Destination $cloudApiStage
    Copy-Item -LiteralPath (Join-Path $projectRoot 'config/cloud_api/README.md') -Destination $cloudApiStage
    Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE') -Destination $portableStage
    Copy-Item -LiteralPath (Join-Path $projectRoot 'THIRD_PARTY_NOTICES.md') -Destination $portableStage
    Copy-Item -LiteralPath (Join-Path $projectRoot 'docs/dependency-licenses.md') -Destination (Join-Path $portableStage 'DEPENDENCY_LICENSES.md')
    Copy-Item -LiteralPath (Join-Path $projectRoot 'third_party/mpv/README.md') -Destination (Join-Path $portableStage 'MPV_RUNTIME.md')
    Copy-Item -LiteralPath (Join-Path $projectRoot 'packaging/windows/NSIS-BUILD-TOOL.md') -Destination $portableStage
    Copy-Item -LiteralPath (Join-Path $projectRoot 'third_party/mpv/manifest.json') -Destination $runtimeStage

    $repository = if ($env:GITHUB_REPOSITORY) { $env:GITHUB_REPOSITORY } else { 'YYDanielJr/yyplayer' }
    $sourceUrl = "https://github.com/$repository/tree/$commit"
    $buildInfo = @"
YYPlayer Windows x64 Release build
Application version: $version
Source commit: $commit
Working tree modified: $([bool]$dirty)
Source: $sourceUrl
Rust target: $target
Application SHA256: $exeHash
libmpv build: $($manifest.build)
libmpv SHA256: $libraryHash
NSIS: $nsisVersion

This package includes third-party components. See LICENSE, THIRD_PARTY_NOTICES.md,
DEPENDENCY_LICENSES.md, MPV_RUNTIME.md, and runtime/manifest.json.
NSIS build-tool attribution and upstream license link are in NSIS-BUILD-TOOL.md.
"@
    [IO.File]::WriteAllText((Join-Path $portableStage 'BUILD-INFO.txt'), $buildInfo, [Text.UTF8Encoding]::new($false))
    $stagedZip = Join-Path $workRoot (Split-Path -Leaf $portableZip)
    $stagedSetup = Join-Path $workRoot (Split-Path -Leaf $setupExe)
    Compress-Archive -Path (Join-Path $portableStage '*') -DestinationPath $stagedZip -CompressionLevel Optimal

    $env:YYPLAYER_PACKAGE_STAGE = $portableStage
    $env:YYPLAYER_PACKAGE_OUTPUT = $workRoot
    $ErrorActionPreference = 'Continue'
    & $compiler "/DAPP_VERSION=$version" "/DAPP_COMMIT=$shortCommit" "/DAPP_ICON=$appIconPath" (Join-Path $projectRoot 'packaging/windows/yyplayer.nsi')
    $nsisExitCode = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($nsisExitCode -ne 0) { throw "NSIS failed with exit code $nsisExitCode." }
    if (-not (Test-Path -LiteralPath $stagedSetup -PathType Leaf)) { throw 'NSIS did not create the expected setup executable.' }

    # Publish only after both formats have been created successfully.
    New-Item -ItemType Directory -Path $distRoot -Force | Out-Null
    Move-Item -LiteralPath $stagedZip -Destination $portableZip
    Move-Item -LiteralPath $stagedSetup -Destination $setupExe
} finally {
    $ErrorActionPreference = 'Stop'
    $env:YYPLAYER_PACKAGE_STAGE = $previousStage
    $env:YYPLAYER_PACKAGE_OUTPUT = $previousOutput
    # Delete only the uniquely named directory created by this invocation.
    $resolvedWorkRoot = [IO.Path]::GetFullPath($workRoot)
    if ((Split-Path -Parent $resolvedWorkRoot).TrimEnd('\') -ne $tempRoot.TrimEnd('\')) { throw 'Unsafe package cleanup path.' }
    if ((Get-Item -LiteralPath $resolvedWorkRoot).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing to clean a package directory replaced by a link.' }
    Remove-Item -LiteralPath $resolvedWorkRoot -Recurse -Force
}

Write-Output "Portable package: $portableZip"
Write-Output "Setup installer: $setupExe"
Write-Output "libmpv SHA256: $libraryHash"
