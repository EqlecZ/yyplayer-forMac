param(
    [Parameter(Mandatory = $true)][string]$Destination,
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'packaging/windows/nsis.json') -Raw | ConvertFrom-Json
$cacheRoot = Join-Path $projectRoot 'target/windows-packaging-tools'
$archive = Join-Path $cacheRoot $manifest.archive_name
$verified = (Test-Path -LiteralPath $archive -PathType Leaf) -and
    (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -eq $manifest.archive_sha256

if (-not $verified) {
    if ($Offline) { throw "The pinned NSIS archive is missing or corrupt: $archive. Run again without -Offline to download it." }
    New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
    $download = Join-Path $cacheRoot ($manifest.archive_name + '.' + [Guid]::NewGuid().ToString('N') + '.partial')
    try {
        Write-Host "Downloading NSIS $($manifest.version)..."
        try { Invoke-WebRequest -UseBasicParsing -Uri $manifest.archive_url -OutFile $download -TimeoutSec 60 }
        catch { Write-Warning "PowerShell download failed: $($_.Exception.Message)" }
        $downloadVerified = (Test-Path -LiteralPath $download -PathType Leaf) -and
            (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant() -eq $manifest.archive_sha256
        if (-not $downloadVerified) {
            # Some SourceForge mirrors return an HTML landing page to PowerShell.
            # Retry the same pinned URL with Windows curl; verify before extraction.
            $curl = Get-Command curl.exe -CommandType Application -ErrorAction SilentlyContinue
            if ($curl) {
                Write-Host 'Retrying the pinned download with curl...'
                $ErrorActionPreference = 'Continue'
                & $curl.Source --fail --location --max-time 60 --output $download $manifest.archive_url
                $curlExitCode = $LASTEXITCODE
                $ErrorActionPreference = 'Stop'
                if ($curlExitCode -ne 0) { throw "NSIS download failed with curl exit code $curlExitCode." }
            }
            if (-not (Test-Path -LiteralPath $download -PathType Leaf) -or
                (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.archive_sha256) {
                throw "NSIS archive checksum mismatch; the download will not be executed. Obtain $($manifest.archive_name) from $($manifest.source) and put the verified archive at $archive."
            }
        }
        Move-Item -LiteralPath $download -Destination $archive -Force
    } finally {
        if (Test-Path -LiteralPath $download) { Remove-Item -LiteralPath $download -Force }
    }
}

# Use a fresh destination each run, rather than trusting previously extracted tools.
if (Test-Path -LiteralPath $Destination) { throw "NSIS extraction destination already exists: $Destination" }
Expand-Archive -LiteralPath $archive -DestinationPath $Destination
$compiler = Join-Path $Destination $manifest.compiler
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) { throw 'The verified NSIS archive does not contain the expected compiler.' }
Write-Output $compiler
