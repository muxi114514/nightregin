<#
  编译并部署到整合包 me3\NightreignPLUS\StatAlloc：
    nr_stat_alloc.dll；nr_stat_alloc.toml 与 schemes.toml 已存在则不覆盖
    -Regulation 时用补丁工具在 More Map 的 regulation 上加组件，生成 pkg\regulation.bin
#>
param(
    [switch]$Regulation,
    [string]$BaseRegulation = "$PSScriptRoot\..\..\me3\NightreignPLUS\More Map and Weapons\regulation.bin"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\.."
$deploy = Join-Path $root "..\me3\NightreignPLUS\StatAlloc"

. (Join-Path $PSScriptRoot "build_env.ps1")
Push-Location $root
try {
    # cargo 把进度写到 stderr，PowerShell 5.1 在 Stop 模式下会误判为错误
    $ErrorActionPreference = "Continue"
    cargo build --release --workspace 2>&1 | ForEach-Object { "$_" } | Where-Object { $_ -notmatch "^\s+Compiling" }
    $code = $LASTEXITCODE
    $ErrorActionPreference = "Stop"
    if ($code -ne 0) { throw "cargo build failed ($code)" }
} finally { Pop-Location }

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root "target" }
New-Item -ItemType Directory -Force $deploy | Out-Null
Copy-Item (Join-Path $targetDir "release\nr_stat_alloc.dll") $deploy -Force

# 配置与方案文件已存在时不覆盖，保留用户的修改
foreach ($name in @("nr_stat_alloc.toml", "schemes.toml")) {
    $dst = Join-Path $deploy $name
    if (-not (Test-Path $dst)) { Copy-Item (Join-Path $root "config\$name") $dst }
}
# 旧版本的数据文件已不再使用（DLL 改为运行时读取游戏参数表）
Remove-Item (Join-Path $deploy "nr_stat_alloc_data.toml") -ErrorAction SilentlyContinue

if ($Regulation) {
    $pkg = Join-Path $deploy "pkg"
    New-Item -ItemType Directory -Force $pkg | Out-Null
    & (Join-Path $targetDir "release\nr_stat_alloc_patcher.exe") $BaseRegulation (Join-Path $pkg "regulation.bin")
    if ($LASTEXITCODE -ne 0) { throw "patcher failed ($LASTEXITCODE)" }
}

Get-ChildItem $deploy -Recurse -File | Where-Object { $_.FullName -notlike "*\logs\*" } | Select-Object FullName, Length, LastWriteTime
