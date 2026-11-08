<#
  生成 N 网发布包：DLL、默认配置、界面素材、原版独立版 pkg、me3 启动配置与整合包补丁工具，
  输出到 <游戏目录>\me3-max\NRStatAlloc-<版本>\ 并压缩成同名 zip。
  目录结构：
    NRStatAlloc.me3                      双击启动（单独使用本模组）
    NRStatAlloc\nr_stat_alloc.dll / nr_stat_alloc.toml / skin\（含 icons\）/ pkg\（regulation.bin、词条用到的 chr / action / msg / sfx、锻造台对话脚本 script）
    patcher\nr_stat_alloc_patcher.exe    整合包作者用：给合并好的 regulation 加组件
  pkg 原样取 tools\build_vanilla_pkg.ps1 的产物（-PkgDir）：补丁加密 IV 随机，独立包与无缝包必须带同一份，
  所以这里不重新生成；游戏或词条改了先重跑 build_vanilla_pkg.ps1。
  不带 schemes.toml：覆盖更新时不会清掉玩家已保存的方案（没有该文件时 DLL 照常运行，面板保存时自动创建）。
  -Seamless：输出 NRStatAlloc-<版本>-Seamless，多带一份无缝联机（SeamlessCoop\，含 nrsc_settings.ini，全员设置需一致），
  NRStatAlloc.me3 直接加载它，不依赖任何整合包。
#>
param(
    [string]$OutRoot = "$PSScriptRoot\..\..\me3-max",
    [string]$PkgDir = "$PSScriptRoot\..\target\pkg_vanilla",
    # 界面素材由 extract_ui.ps1 生成在整合包部署目录
    [string]$SkinDir = "$PSScriptRoot\..\..\me3\NightreignPLUS\StatAlloc\skin",
    [switch]$Seamless,
    # 无缝联机来源，其中的 nrsc_settings.ini（敌人倍率）原样带上
    [string]$SeamlessDir = "$PSScriptRoot\..\..\me3\MMV\me3\SeamlessCoop"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\.."

. (Join-Path $PSScriptRoot "build_env.ps1")
Push-Location $root
try {
    $ErrorActionPreference = "Continue"
    cargo build --release --workspace 2>&1 | ForEach-Object { "$_" } | Where-Object { $_ -notmatch "^\s+Compiling" }
    $code = $LASTEXITCODE
    $ErrorActionPreference = "Stop"
    if ($code -ne 0) { throw "cargo build failed ($code)" }
} finally { Pop-Location }

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root "target" }
$version = ([regex]::Match((Get-Content (Join-Path $root "Cargo.toml") -Raw), '(?m)^version\s*=\s*"([^"]+)"')).Groups[1].Value
if (-not (Test-Path "$SkinDir\button.png") -or -not (Test-Path "$SkinDir\icons\*.png")) { throw "skin or weapon icons not found: $SkinDir (run tools\extract_ui.ps1 first)" }
if (-not (Test-Path "$PkgDir\regulation.bin")) { throw "standalone pkg not found: $PkgDir (run tools\build_vanilla_pkg.ps1 first)" }
$PkgDir = (Resolve-Path $PkgDir).Path

New-Item -ItemType Directory -Force $OutRoot | Out-Null
$OutRoot = (Resolve-Path $OutRoot).Path
$stage = Join-Path $OutRoot ("NRStatAlloc-$version" + $(if ($Seamless) { "-Seamless" } else { "" }))
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
$mod = Join-Path $stage "NRStatAlloc"
New-Item -ItemType Directory -Force "$mod\pkg", "$mod\skin\icons", "$stage\patcher" | Out-Null

Copy-Item (Join-Path $targetDir "release\nr_stat_alloc.dll") $mod
Copy-Item (Join-Path $root "config\nr_stat_alloc.toml") $mod
Copy-Item "$SkinDir\*.png" "$mod\skin"
Copy-Item "$SkinDir\icons\*.png" "$mod\skin\icons"
Copy-Item (Join-Path $targetDir "release\nr_stat_alloc_patcher.exe") "$stage\patcher"
Get-ChildItem $PkgDir -Recurse -File | Where-Object { $_.Extension -ne ".tmp" } | ForEach-Object {
    $dst = Join-Path "$mod\pkg" $_.FullName.Substring($PkgDir.Length + 1)
    New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
    Copy-Item -LiteralPath $_.FullName $dst
}

if ($Seamless) {
    if (-not (Test-Path "$SeamlessDir\nrsc.dll")) { throw "Seamless Co-op not found: $SeamlessDir" }
    Copy-Item (Resolve-Path $SeamlessDir).Path "$stage\SeamlessCoop" -Recurse
    # Smithbox 工程文件、日志不属于无缝联机本体
    Get-ChildItem "$stage\SeamlessCoop" -Recurse -File -Include project.json, *.log | Remove-Item -Force
}

# me3 启动配置：路径相对本文件；单独包与无缝联机一起用时把 natives / packages 两项抄进自己的配置即可
# 存档用无缝联机的 .co2，不碰官服 .sl2（游戏会把当前 regulation 写进存档）
$seamlessNative = if ($Seamless) { "`n[[natives]]`npath = `"./SeamlessCoop/nrsc.dll`"`nload_early = true`n" } else { "" }
$nrscOptional = if ($Seamless) { "false" } else { "true" }
$me3Profile = @"
profileVersion = "v1"
savefile = "NR0000.co2"
start_online = false

[[supports]]
game = "nightreign"
$seamlessNative
[[natives]]
path = "./NRStatAlloc/nr_stat_alloc.dll"
load_after = [{ id = "nrsc.dll", optional = $nrscOptional }]

[[packages]]
id = "NRStatAlloc"
source = "./NRStatAlloc/pkg"
"@
[IO.File]::WriteAllText("$stage\NRStatAlloc.me3", $me3Profile, (New-Object System.Text.UTF8Encoding($false)))

$zip = "$stage.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$stage\*" -DestinationPath $zip
Get-ChildItem $stage -Recurse -File | Select-Object @{ n = "File"; e = { $_.FullName.Substring($stage.Length + 1) } }, Length
Get-Item $zip | Select-Object FullName, Length
