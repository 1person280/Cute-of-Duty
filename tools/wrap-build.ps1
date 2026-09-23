# 使用方法：
#   .\tools\wrap-build.ps1 build --features demo        # 代理主包编译（限并发+进度条）
#   .\tools\wrap-build.ps1 --release checkout --features demo
#   $env:CARGO_WRAP_JOBS=2; .\tools\wrap-build.ps1 build  # 临时限 2 并发
#
# 原理：
#   1) 首次用独立 workspace 编译 tools/cargo-wrap（仅 indicatif/colored/regex，秒级，不占主包缓存）；
#   2) 调用 cargo-wrap.exe 真正接管 cargo，强制 -j 限并行、结构化日志、进度条。
# 目标目录固定为 tools/cargo-wrap/target，与主包 target 完全隔离。
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$ToolDir = Join-Path $Root "tools\cargo-wrap"
$Exe = Join-Path $ToolDir "target\release\cargo-wrap.exe"

if (-not (Test-Path $Exe)) {
    Write-Host "[wrap-build] 首次运行：编译 tools/cargo-wrap（独立 target，仅依赖 indicatif/colored/regex）..." -ForegroundColor Cyan
    Push-Location $ToolDir
    try {
        & cargo build --release
        if ($LASTEXITCODE -ne 0) {
            Write-Error "cargo-wrap 编译失败（退出码 $LASTEXITCODE）"
            exit $LASTEXITCODE
        }
    } finally {
        Pop-Location
    }
}

# 把剩余参数透传给代理工具；空参数时给出提示。
if ($CargoArgs.Count -eq 0) {
    Write-Host "用法: .\tools\wrap-build.ps1 <cargo 子命令及参数>，例如 build --features demo" -ForegroundColor Yellow
    exit 2
}

& $Exe @CargoArgs
exit $LASTEXITCODE