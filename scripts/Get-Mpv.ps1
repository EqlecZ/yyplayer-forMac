param([string]$Destination = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'third_party/mpv/manifest.json') -Raw | ConvertFrom-Json
if (-not $Destination) { $Destination = Join-Path $projectRoot 'third_party/mpv/windows-x64' }
$runtimeDirectory = [IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Path $runtimeDirectory -Force | Out-Null
$archivePath = Join-Path $runtimeDirectory 'runtime.7z'
Invoke-WebRequest -UseBasicParsing -Uri $manifest.archive_url -OutFile $archivePath
if ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.archive_sha256) { throw 'libmpv archive checksum mismatch' }
$sevenZip = Get-Command 7z.exe -CommandType Application -ErrorAction SilentlyContinue
if (-not $sevenZip) {
    foreach ($installRoot in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
        if ($installRoot) {
            $candidate = Join-Path $installRoot '7-Zip/7z.exe'
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { $sevenZip = Get-Command $candidate -CommandType Application; break }
        }
    }
}
if ($sevenZip) {
    & $sevenZip.Source x $archivePath "-o$runtimeDirectory" -y | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Extraction failed with 7-Zip after the archive checksum passed.' }
} else {
    & tar.exe -xf $archivePath -C $runtimeDirectory
    if ($LASTEXITCODE -ne 0) { throw 'Extraction failed. Install 7-Zip or use a tar.exe build with 7z support.' }
}
$libraryPath = Join-Path $runtimeDirectory $manifest.library
if (-not (Test-Path -LiteralPath $libraryPath)) { throw 'Expected library missing from archive' }
$libraryHash = (Get-FileHash -LiteralPath $libraryPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($libraryHash -ne $manifest.library_sha256) { throw 'libmpv DLL checksum mismatch' }
Write-Output "Runtime ready: $libraryPath"
Write-Output "DLL SHA256: $libraryHash"
