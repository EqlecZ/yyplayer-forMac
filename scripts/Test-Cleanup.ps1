$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$testRoot = Join-Path $root ('target/cleanup-qa-' + [Guid]::NewGuid().ToString('N'))
if (-not ([IO.Path]::GetFullPath($testRoot)).StartsWith(([IO.Path]::GetFullPath($root) + '\'), [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe fixture root' }
$workspace = Join-Path $testRoot 'workspace'
$preserved = @(
    'Cargo.toml', 'src/main.rs', 'docs/status.md',
    'target/release/yyplayer.exe', 'target/release/runtime/libmpv-2.dll',
    'target/release/config/cloud_api/custom.json', 'target/debug/config/cloud_api/custom.json',
    'third_party/mpv/windows-x64/libmpv-2.dll', 'third_party/mpv/windows-x64/include/client.h'
)
$removed = @(
    'target/debug/yyplayer.exe', 'target/debug/deps/cache.obj',
    'target/release/deps/cache.obj', 'target/release/yyplayer.pdb',
    'target/fixtures/data.bin', 'third_party/mpv/windows-x64/runtime.7z',
    'third_party/mpv/windows-x64/libmpv.dll.a'
)
$hashes = @{}
foreach ($path in ($preserved + $removed)) {
    $absolute = Join-Path $workspace $path
    New-Item -ItemType Directory -Path (Split-Path -Parent $absolute) -Force | Out-Null
    [IO.File]::WriteAllText($absolute, ('Own cleanup fixture: ' + $path), [Text.UTF8Encoding]::new($false))
    $hashes[$path] = (Get-FileHash -LiteralPath $absolute -Algorithm SHA256).Hash
}
$scriptDir = Join-Path $workspace 'scripts'
New-Item -ItemType Directory -Path $scriptDir -Force | Out-Null
$cleaner = Join-Path $scriptDir 'Clean-Workspace.ps1'
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Clean-Workspace.ps1') -Destination $cleaner
& $cleaner -WhatIf | Out-Null
foreach ($path in ($preserved + $removed)) {
    if ((Get-FileHash -LiteralPath (Join-Path $workspace $path) -Algorithm SHA256).Hash -ne $hashes[$path]) { throw "WhatIf changed $path" }
}
& $cleaner | Out-Null
foreach ($path in $preserved) {
    if ((Get-FileHash -LiteralPath (Join-Path $workspace $path) -Algorithm SHA256).Hash -ne $hashes[$path]) { throw "Cleanup changed protected $path" }
}
foreach ($path in $removed) {
    if (Test-Path -LiteralPath (Join-Path $workspace $path)) { throw "Generated file retained: $path" }
}
$evidence = Join-Path $root 'target/cleanup-results.json'
@{ result='PASS'; whatif_unchanged=$true; preserved=$preserved; removed=$removed; fixture=$testRoot } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $evidence -Encoding UTF8
Write-Output 'PASS: WhatIf leaves all files unchanged; cleanup removes generated files and preserves executable, both API config directories, runtime, headers, source and docs.'
