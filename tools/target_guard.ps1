# target_guard.ps1 - build artifact size guard
#
# Why: this repo has a strict disk budget: `target/` must stay <= 4GB. Cargo has no size cap,
# so this script checks the size BEFORE a build and, when over the limit, removes TRANSIENT
# artifacts (debug / flycheck / tmp / doc) while KEEPING `target/release` - the only cache
# worth reusing, which avoids a full-tree rebuild.
#
# Usage (run before building; Windows PowerShell 5.1 is fine, no pwsh needed):
#     powershell -File tools/target_guard.ps1            # check/clean with the default 4GB cap
#     powershell -File tools/target_guard.ps1 -LimitGB 3 # custom cap
#     powershell -File tools/target_guard.ps1 -WhatIf    # report only, delete nothing
#
# Exit code: 0 = within limit; 1 = still over the limit after cleanup (run `cargo clean` or raise -LimitGB).
#
# Note: keep this file ASCII-only - Windows PowerShell 5.1 reads BOM-less scripts as ANSI.

[CmdletBinding()]
param(
    [double]$LimitGB = 4.0,
    [switch]$WhatIf
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$targetDir = Join-Path $repoRoot 'target'

function Get-DirBytes([string]$path) {
    if (-not (Test-Path $path)) { return 0 }
    return [double](Get-ChildItem $path -Recurse -File -ErrorAction SilentlyContinue |
        Measure-Object Length -Sum).Sum
}

function Format-GB([double]$bytes) { '{0:N2} GB' -f ($bytes / 1GB) }

if (-not (Test-Path $targetDir)) {
    Write-Host '[target_guard] target/ does not exist; nothing to do.'
    exit 0
}

$limitBytes = $LimitGB * 1GB
$before = Get-DirBytes $targetDir
Write-Host ('[target_guard] target/ = {0} (limit {1:N2} GB)' -f (Format-GB $before), $LimitGB)

if ($before -le $limitBytes) {
    Write-Host '[target_guard] within limit; OK.'
    exit 0
}

# Over limit: drop transient artifacts first, keep the release reuse cache.
$transient = @('debug', 'flycheck0', 'tmp', 'doc') |
    ForEach-Object { Join-Path $targetDir $_ } |
    Where-Object { Test-Path $_ }

foreach ($dir in $transient) {
    if ($WhatIf) {
        Write-Host ('[target_guard] (WhatIf) would delete {0} ({1})' -f $dir, (Format-GB (Get-DirBytes $dir)))
    } else {
        Write-Host ('[target_guard] cleaning {0} ({1})' -f $dir, (Format-GB (Get-DirBytes $dir)))
        Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
    }
}

if (-not $WhatIf) {
    $after = Get-DirBytes $targetDir
    Write-Host ('[target_guard] after cleanup target/ = {0}' -f (Format-GB $after))
    if ($after -gt $limitBytes) {
        Write-Warning ('[target_guard] still over limit: the release reuse cache alone exceeds {0:N2} GB; run cargo clean or raise -LimitGB.' -f $LimitGB)
        exit 1
    }
}

Write-Host '[target_guard] back within limit; OK.'
exit 0