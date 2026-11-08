<#
  生成整合包专用发布包：<游戏目录>\me3-max\NRStatAlloc-<版本>-<后缀>\（+ 同名 zip），解压到整合包根目录即可。
    -Target MmvKongCheng（默认）：MMV 整合包根目录 me3\MMV\me3，后缀 MMV-KongCheng；
      带空城包的在线版 / 无缝版用 pkg，不带空城包的无缝版用 pkg_mmv（build_mmv_pkg.ps1 -Target Mmv）
    -Target Mmv：同一整合包，只打不带空城包的部分（pkg_mmv + 对应启动配置），后缀 MMV
    -Target KongCheng：空城 0.46 独立包，整合包根目录 me3\kongcheng，后缀 KongCheng-0.46
  目录结构：
    StatAlloc\nr_stat_alloc.dll / nr_stat_alloc.toml / skin\（含 icons\）/ pkg*\（regulation.bin、词条用到的 chr / action / msg / sfx、锻造台对话脚本 script）
    属性分配启动配置 .me3（取自整合包目录）
  pkg 直接用整合包 StatAlloc 里那份（tools\build_mmv_pkg.ps1 的产物）：私服按 regulation 哈希分匹配池，所有人必须是同一个文件，不能重新生成。
  不带 schemes.toml：覆盖更新时保留朋友已保存的方案（没有该文件时 DLL 照常运行，面板保存时自动创建）。
  空城 0.46 的 nr_stat_alloc.toml 取整合包里部署的那份（关掉了与伤害统计冲突的 F7~F10 调试键）；
  MMV 用模板并关掉锻造石【３】金色强化（MMV 自带 RestoredLegendaryStones.dll）。
#>
param(
    [ValidateSet("MmvKongCheng", "Mmv", "KongCheng")]
    [string]$Target = "MmvKongCheng",
    [string]$PackDir,
    [string]$OutRoot = "$PSScriptRoot\..\..\me3-max"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\.."
# Profiles：包里的文件名 -> 整合包目录里按顺序找的来源（本机把空城包无缝版改名成了 wufengshuxing.me3）
$targets = @{
    MmvKongCheng = @{ Dir = "me3\MMV\me3"; Suffix = "MMV-KongCheng"; DeployedConfig = $false; LegendaryOff = $true; Pkgs = @("pkg", "pkg_mmv")
                      Profiles = [ordered]@{
                          "空城包在线版 + 属性分配.me3" = @("空城包在线版 + 属性分配.me3")
                          "空城包无缝版 + 属性分配.me3" = @("空城包无缝版 + 属性分配.me3", "wufengshuxing.me3")
                          "MMV无缝版（不带空城）+ 属性分配.me3" = @("MMV无缝版（不带空城）+ 属性分配.me3") } }
    Mmv          = @{ Dir = "me3\MMV\me3"; Suffix = "MMV"; DeployedConfig = $false; LegendaryOff = $true; Pkgs = @("pkg_mmv")
                      Profiles = [ordered]@{ "MMV无缝版（不带空城）+ 属性分配.me3" = @("MMV无缝版（不带空城）+ 属性分配.me3") } }
    KongCheng    = @{ Dir = "me3\kongcheng"; Suffix = "KongCheng-0.46"; DeployedConfig = $true; LegendaryOff = $false; Pkgs = @("pkg")
                      Profiles = [ordered]@{ "点我启动游戏_无缝联机【可6人】+ 属性分配.me3" = @("点我启动游戏_无缝联机【可6人】+ 属性分配.me3") } }
}
$t = $targets[$Target]
if (-not $PackDir) { $PackDir = Join-Path "$PSScriptRoot\..\.." $t.Dir }

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
$PackDir = (Resolve-Path $PackDir).Path
$skin = "$PackDir\StatAlloc\skin"
if (-not (Test-Path "$skin\button.png") -or -not (Test-Path "$skin\icons\*.png")) { throw "skin or weapon icons not found: $skin (run tools\extract_ui.ps1 first)" }
foreach ($p in $t.Pkgs) {
    if (-not (Test-Path "$PackDir\StatAlloc\$p\regulation.bin")) { throw "patched regulation not found: $PackDir\StatAlloc\$p\regulation.bin" }
}
# 包里的配置名 -> 实际来源路径
$profileSources = [ordered]@{}
foreach ($name in $t.Profiles.Keys) {
    $src = $t.Profiles[$name] | ForEach-Object { Join-Path $PackDir $_ } | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $src) { throw "profile not found: $name ($($t.Profiles[$name] -join ' / '))" }
    $profileSources[$name] = $src
}

New-Item -ItemType Directory -Force $OutRoot | Out-Null
$stage = Join-Path (Resolve-Path $OutRoot).Path "NRStatAlloc-$version-$($t.Suffix)"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
$mod = Join-Path $stage "StatAlloc"
New-Item -ItemType Directory -Force "$mod\skin\icons" | Out-Null

Copy-Item (Join-Path $targetDir "release\nr_stat_alloc.dll") $mod
$config = if ($t.DeployedConfig) { "$PackDir\StatAlloc\nr_stat_alloc.toml" } else { Join-Path $root "config\nr_stat_alloc.toml" }
Copy-Item -LiteralPath $config $mod
if ($t.LegendaryOff) {
    $cfgPath = Join-Path $mod "nr_stat_alloc.toml"
    $text = [IO.File]::ReadAllText($cfgPath)
    $nl = if ($text.Contains("`r`n")) { "`r`n" } else { "`n" }
    # [legendary_upgrade] 段里的 enabled = true → 加一行说明并改为 false
    $edited = [regex]::Replace($text, '(?m)(^\[legendary_upgrade\]\r?\n(?:#.*\r?\n)*)enabled\s*=\s*true',
        '${1}# MMV 自带 RestoredLegendaryStones.dll 已实现，这里关掉' + $nl + 'enabled = false')
    if ($edited -eq $text) { throw "cannot disable [legendary_upgrade] in $cfgPath" }
    [IO.File]::WriteAllText($cfgPath, $edited, (New-Object System.Text.UTF8Encoding $false))
}
Copy-Item "$skin\*.png" "$mod\skin"
Copy-Item "$skin\icons\*.png" "$mod\skin\icons"
# 各 pkg 整个目录，跳过首次打补丁留下的备份
foreach ($p in $t.Pkgs) {
    $src = "$PackDir\StatAlloc\$p"
    Get-ChildItem $src -Recurse -File | Where-Object { $_.Name -notlike "*.before-*" -and $_.Extension -ne ".tmp" } | ForEach-Object {
        $dst = Join-Path "$mod\$p" $_.FullName.Substring($src.Length + 1)
        New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
        Copy-Item -LiteralPath $_.FullName $dst
    }
}
foreach ($name in $profileSources.Keys) { Copy-Item -LiteralPath $profileSources[$name] (Join-Path $stage $name) }

$zip = "$stage.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$stage\*" -DestinationPath $zip
Get-ChildItem $stage -Recurse -File | Select-Object @{ n = "File"; e = { $_.FullName.Substring($stage.Length + 1) } }, Length
Get-Item $zip | Select-Object FullName, Length
