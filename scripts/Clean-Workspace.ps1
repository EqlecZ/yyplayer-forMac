[CmdletBinding(SupportsShouldProcess = $true)]
param()
$ErrorActionPreference = 'Stop'
$workspaceRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot)).TrimEnd('\')
if (-not (Test-Path -LiteralPath (Join-Path $workspaceRoot 'Cargo.toml'))) { throw 'Not a YYPlayer workspace' }
$targetRoot = Join-Path $workspaceRoot 'target'
$releaseRoot = Join-Path $targetRoot 'release'
$debugRoot = Join-Path $targetRoot 'debug'
$debugConfig = Join-Path $debugRoot 'config'
$executable = Join-Path $releaseRoot 'yyplayer.exe'
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw 'Build release first; no executable to preserve' }

# Never follow a directory junction or accept a deletion outside the workspace.
function Assert-WorkspacePath([string]$candidate) {
    $absolute = [IO.Path]::GetFullPath($candidate)
    if (-not $absolute.StartsWith($workspaceRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw "Unsafe cleanup path: $absolute" }
    $ancestor = Split-Path -Parent $absolute
    while ($ancestor -and $ancestor.Length -ge $workspaceRoot.Length) {
        if ((Get-Item -LiteralPath $ancestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Refusing linked ancestor: $ancestor" }
        if ($ancestor -eq $workspaceRoot) { break }
        $ancestor = Split-Path -Parent $ancestor
    }
    if (Test-Path -LiteralPath $absolute) {
        $entry = Get-Item -LiteralPath $absolute -Force
        if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Refusing linked cleanup target: $absolute" }
        if ($entry.PSIsContainer) {
            $links = Get-ChildItem -LiteralPath $absolute -Recurse -Force | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }
            if ($links) { throw "Refusing recursive cleanup containing links: $absolute" }
        }
    }
    return $absolute
}
$candidates = @(
    # Executable-adjacent config/cloud_api may contain user API JSON files.
    # When present, clean debug's children while retaining this data directory.
    Get-ChildItem -LiteralPath $targetRoot -Force | Where-Object { $_.FullName -ne $releaseRoot -and ($_.FullName -ne $debugRoot -or -not (Test-Path -LiteralPath $debugConfig)) } | ForEach-Object { $_.FullName }
    if (Test-Path -LiteralPath $debugConfig) {
        Get-ChildItem -LiteralPath $debugRoot -Force | Where-Object { $_.FullName -ne $debugConfig } | ForEach-Object { $_.FullName }
    }
    Get-ChildItem -LiteralPath $releaseRoot -Force | Where-Object { $_.FullName -ne $executable -and $_.Name -notin @('runtime', 'config') } | ForEach-Object { $_.FullName }
    Join-Path $workspaceRoot 'third_party/mpv/windows-x64/runtime.7z'
    Join-Path $workspaceRoot 'third_party/mpv/windows-x64/libmpv.dll.a'
)
$checked = @($candidates | ForEach-Object { Assert-WorkspacePath $_ })
$removedBytes = 0L
$failed = @()
foreach ($path in $checked) {
    if (-not (Test-Path -LiteralPath $path)) { continue }
    $entry = Get-Item -LiteralPath $path -Force
    $bytes = if ($entry.PSIsContainer) { (Get-ChildItem -LiteralPath $path -File -Recurse -Force | Measure-Object Length -Sum).Sum } else { $entry.Length }
    if ($PSCmdlet.ShouldProcess($path, 'Remove generated files')) {
        try { Remove-Item -LiteralPath $path -Recurse -Force; $removedBytes += $bytes }
        catch { $failed += $path; Write-Warning "Could not remove $path : $($_.Exception.Message)" }
    }
}
Write-Output "Removed bytes: $removedBytes"
Write-Output "Preserved executable: $executable"
Write-Output 'Source, Git history, docs, executable-adjacent API config, user settings/media and fixed libmpv DLL/headers preserved.'
if ($failed.Count) { throw "Cleanup incomplete; locked targets: $($failed -join ', ')" }
