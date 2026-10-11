$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$runtimeRoot = Join-Path $projectRoot 'third_party/mpv/windows-x64'
$appIconPath = Join-Path $projectRoot 'assets/icons/yyplayer/liquid-orbit-disc/yyplayer.ico'
$exePath = Join-Path $projectRoot 'target/x86_64-pc-windows-msvc/release/yyplayer.exe'
$distRoot = Join-Path $projectRoot 'dist'
$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$portableStage = Join-Path $tempRoot 'yyplayer-windows-x64-portable'

if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) { throw "Release executable not found: $exePath" }
if (-not (Test-Path -LiteralPath $appIconPath -PathType Leaf)) { throw "Windows application icon not found: $appIconPath" }
if (-not (Test-Path -LiteralPath (Join-Path $runtimeRoot 'libmpv-2.dll') -PathType Leaf)) { throw 'The verified mpv runtime has not been extracted.' }
if (Test-Path -LiteralPath $portableStage) { throw "Refusing to overwrite existing staging directory: $portableStage" }
New-Item -ItemType Directory -Path $distRoot -Force | Out-Null
New-Item -ItemType Directory -Path $portableStage | Out-Null
$portableName = 'YYPlayer'
$compiler = (Get-Command makensis.exe -ErrorAction Stop).Source
$nsisVersion = (Get-Item -LiteralPath $compiler).VersionInfo.FileVersion

$cargoToml = Get-Content -LiteralPath (Join-Path $projectRoot 'Cargo.toml') -Raw
if ($cargoToml -notmatch '(?ms)^\[workspace\.package\]\s+.*?^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"') {
    throw 'Could not read the three-part workspace version from Cargo.toml.'
}
$version = $Matches[1]
$commit = $env:GITHUB_SHA
if (-not $commit) { $commit = (git -C $projectRoot rev-parse HEAD).Trim() }
$shortCommit = $commit.Substring(0, [Math]::Min(12, $commit.Length))
$portableZip = Join-Path $distRoot "$portableName-$version-$shortCommit-windows-x64-portable.zip"
$setupExe = Join-Path $distRoot "$portableName-$version-$shortCommit-windows-x64-setup.exe"
if ((Test-Path -LiteralPath $portableZip) -or (Test-Path -LiteralPath $setupExe)) { throw 'A package for this commit already exists in dist.' }

Copy-Item -LiteralPath $exePath -Destination (Join-Path $portableStage 'yyplayer.exe')
$runtimeStage = Join-Path $portableStage 'runtime'
New-Item -ItemType Directory -Path $runtimeStage | Out-Null
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'third_party/mpv/manifest.json') -Raw | ConvertFrom-Json
$library = Join-Path $runtimeRoot $manifest.library
$libraryHash = (Get-FileHash -LiteralPath $library -Algorithm SHA256).Hash.ToLowerInvariant()
if ($libraryHash -ne $manifest.library_sha256) { throw 'Extracted libmpv DLL does not match the pinned manifest hash.' }

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

$sourceUrl = "https://github.com/$env:GITHUB_REPOSITORY/tree/$commit"
$buildInfo = @"
YYPlayer Windows x64 Release build
Application version: $version
Source commit: $commit
Source: $sourceUrl
Rust target: x86_64-pc-windows-msvc
libmpv build: $($manifest.build)
libmpv SHA256: $libraryHash
NSIS: $nsisVersion

This package includes third-party components. See LICENSE, THIRD_PARTY_NOTICES.md,
DEPENDENCY_LICENSES.md, MPV_RUNTIME.md, and runtime/manifest.json.
NSIS build-tool attribution and upstream license link are in NSIS-BUILD-TOOL.md.
"@
[IO.File]::WriteAllText((Join-Path $portableStage 'BUILD-INFO.txt'), $buildInfo, [Text.UTF8Encoding]::new($false))
Compress-Archive -Path (Join-Path $portableStage '*') -DestinationPath $portableZip -CompressionLevel Optimal

$env:YYPLAYER_PACKAGE_STAGE = $portableStage
$env:YYPLAYER_PACKAGE_OUTPUT = $distRoot
& $compiler "/DAPP_VERSION=$version" "/DAPP_COMMIT=$shortCommit" "/DAPP_ICON=$appIconPath" (Join-Path $projectRoot 'packaging/windows/yyplayer.nsi')
if ($LASTEXITCODE -ne 0) { throw "NSIS failed with exit code $LASTEXITCODE." }
if (-not (Test-Path -LiteralPath $setupExe -PathType Leaf)) { throw 'NSIS did not create the expected setup executable.' }

Write-Output "Portable package: $portableZip"
Write-Output "Setup installer: $setupExe"
Write-Output "libmpv SHA256: $libraryHash"
