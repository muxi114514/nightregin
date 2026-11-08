<#
  启动器全部测试：cargo test --workspace，再跑前端探测（有 ui\probes\run.mjs 且没给 -NoProbe 时）
  用法：powershell -ExecutionPolicy Bypass -File launcher\test.ps1 [-Filter <名字>] [-NoProbe]
  app crate 编译时内嵌 ui\dist：新树上还没有时先做一次普通前端构建
#>
param(
    [string]$Filter = '',
    [switch]$NoProbe
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '..\tools\build_env.ps1')
$env:CARGO_TARGET_DIR = if ($env:NRGA_TARGET_DIR) { $env:NRGA_TARGET_DIR } else { 'D:\DevTools\nrga-launcher-target' }
if ($env:CARGO_TARGET_DIR.Contains(' ')) {
    throw "CARGO_TARGET_DIR 不能含空格（windres 预处理会在空格处切断路径）：$env:CARGO_TARGET_DIR"
}

# 运行原生命令，只按退出码判成败（PS 5.1 可能把 stderr 输出包装成错误记录，不能让它中断脚本）
function Invoke-Native([string]$What, [scriptblock]$Command) {
    $ErrorActionPreference = 'Continue'
    # 先清空：命令没能启动时 $LASTEXITCODE 不会更新，不能沿用上一条命令的退出码
    $global:LASTEXITCODE = $null
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$What 失败（退出码 $LASTEXITCODE）" }
}

$ui = Join-Path $PSScriptRoot 'ui'

# 0. 新树上还没有 ui\dist：app crate 的 generate_context! 要内嵌它，先做一次普通构建
if (-not (Test-Path (Join-Path $ui 'dist\index.html'))) {
    Push-Location $ui
    try {
        if (-not (Test-Path 'node_modules')) { Invoke-Native 'npm ci' { npm.cmd ci } }
        Invoke-Native 'npm run build' { npm.cmd run build }
    } finally { Pop-Location }
}

# 1. Rust 测试
$cargoArgs = @('test', '--workspace')
if ($Filter) { $cargoArgs += $Filter }
Push-Location $PSScriptRoot
try { Invoke-Native 'cargo test' { cargo @cargoArgs } }
finally { Pop-Location }

# 1.5 clippy：告警即失败；按名字过滤时跳过，免得每次小范围测试都多跑一遍
if (-not $Filter) {
    Push-Location $PSScriptRoot
    try { Invoke-Native 'cargo clippy' { cargo clippy --workspace --all-targets -- -D warnings } }
    finally { Pop-Location }
}

# 2. 前端探测：探测构建额外打包 probe.html，再由 run.mjs 逐个运行
if (-not $NoProbe -and (Test-Path (Join-Path $ui 'probes\run.mjs'))) {
    Push-Location $ui
    $env:NRGA_PROBE = '1'
    try {
        Invoke-Native 'npm run build（探测构建）' { npm.cmd run build }
        $nodeArgs = @('probes\run.mjs')
        if ($Filter) { $nodeArgs += $Filter }
        Invoke-Native '前端探测' { node @nodeArgs }
    } finally {
        Remove-Item Env:NRGA_PROBE -ErrorAction SilentlyContinue
        Pop-Location
    }
}

Write-Output 'ALL TESTS PASSED'
