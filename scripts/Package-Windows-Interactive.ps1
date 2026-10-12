param(
    [switch]$Offline,
    [switch]$NoOpen,
    [string]$NsisCompiler = ''
)

$ErrorActionPreference = 'Stop'
# Cargo emits UTF-8 JSON; Explorer's legacy console code page can otherwise
# corrupt artifact paths containing Chinese when Windows PowerShell is used.
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$OutputEncoding = [Console]::OutputEncoding
$projectRoot = Split-Path -Parent $PSScriptRoot
$runId = (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 8)
$runRoot = Join-Path $projectRoot "target/windows-packaging-runs/$runId"
$outputDirectory = Join-Path $projectRoot "dist/windows-$runId"
$logPath = Join-Path $runRoot 'packaging.log'
$exitCode = 1
$transcribing = $false

try {
    if ($env:OS -ne 'Windows_NT') { throw 'This launcher requires Windows x64.' }
    New-Item -ItemType Directory -Path $runRoot -Force | Out-Null
    Start-Transcript -LiteralPath $logPath | Out-Null
    $transcribing = $true
    Write-Host 'YYPlayer Windows packaging' -ForegroundColor Cyan
    Write-Host "Project: $projectRoot"
    Write-Host "Log: $logPath"

    # Explorer need not inherit a terminal's PATH. Include the normal user Rust installation.
    if (-not (Get-Command cargo.exe -CommandType Application -ErrorAction SilentlyContinue)) {
        $cargoRoot = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
        $cargoBin = Join-Path $cargoRoot 'bin'
        if (Test-Path -LiteralPath (Join-Path $cargoBin 'cargo.exe') -PathType Leaf) { $env:PATH = "$cargoBin;$env:PATH" }
    }
    if (-not (Get-Command cargo.exe -CommandType Application -ErrorAction SilentlyContinue)) {
        throw 'Rust/Cargo is missing. Install the Rust x64 MSVC toolchain and Visual Studio C++ Build Tools with Windows SDK, then double-click Package-Windows.cmd again.'
    }
    if (-not (Get-Command git.exe -CommandType Application -ErrorAction SilentlyContinue)) {
        foreach ($installRoot in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:LOCALAPPDATA)) {
            if ($installRoot) {
                $gitBin = Join-Path $installRoot 'Git/cmd'
                if (Test-Path -LiteralPath (Join-Path $gitBin 'git.exe') -PathType Leaf) { $env:PATH = "$gitBin;$env:PATH"; break }
            }
        }
    }
    if (-not (Get-Command git.exe -CommandType Application -ErrorAction SilentlyContinue)) { throw 'Git is missing. Install Git for Windows, then run the launcher again.' }

    Write-Host '[1/3] Checking the pinned libmpv runtime...'
    $manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'third_party/mpv/manifest.json') -Raw | ConvertFrom-Json
    $library = Join-Path $projectRoot ('third_party/mpv/windows-x64/' + $manifest.library)
    if (-not (Test-Path -LiteralPath $library -PathType Leaf)) {
        if ($Offline) { throw 'The libmpv runtime is missing. Run again without -Offline to download it.' }
        & (Join-Path $PSScriptRoot 'Get-Mpv.ps1')
    }
    if ((Get-FileHash -LiteralPath $library -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.library_sha256) {
        throw 'The libmpv DLL does not match its pinned hash. Run scripts/Get-Mpv.ps1 to restore the verified runtime.'
    }

    Write-Host '[2/3] Preparing the installer compiler...'
    if (-not $NsisCompiler) {
        $NsisCompiler = & (Join-Path $PSScriptRoot 'Get-Nsis.ps1') -Destination (Join-Path $runRoot 'nsis') -Offline:$Offline
    }

    Write-Host '[3/3] Compiling Release and creating both packages...'
    & (Join-Path $PSScriptRoot 'Package-Windows.ps1') -Offline:$Offline -NsisCompiler $NsisCompiler -OutputDirectory $outputDirectory
    Write-Host ''
    Write-Host 'SUCCESS: portable ZIP and Setup installer are ready.' -ForegroundColor Green
    Write-Host "Output: $outputDirectory"
    Write-Host "Log: $logPath"
    $exitCode = 0
    if (-not $NoOpen) {
        try {
            # Explorer is intentionally visible: show the completed artifacts.
            Start-Process -FilePath explorer.exe -ArgumentList ('"' + $outputDirectory + '"') | Out-Null
        } catch {
            Write-Warning "Packages were created, but the output folder could not be opened: $($_.Exception.Message)"
        }
    }
} catch {
    Write-Host ''
    Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Log: $logPath"
} finally {
    if ($transcribing) { Stop-Transcript | Out-Null }
}

exit $exitCode
