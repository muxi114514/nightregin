<#
  生成 MMV 系整合包的属性分配 pkg（属性分配启动配置都最后加载它）。-Target 选整合包：
    MmvKongCheng（默认）me3\MMV\me3 的 MMV + 空城包，底包 modKongCheng，输出 StatAlloc\pkg
    Mmv                  同目录只用 MMV、不带空城包，底包 mod，输出 StatAlloc\pkg_mmv
    KongCheng            me3\kongcheng 的空城 0.46 独立包，底包 mod，输出 me3\kongcheng\StatAlloc\pkg
    regulation.bin       <底包>\regulation.bin → mmv_rebalance（首领重平衡，-NoRebalance 跳过）→ mmv_relics（自定义遗物词条）
                         → nr_stat_alloc_patcher（属性组件）。重平衡的规则 2 与 MMV 本体比对，需要本机装有 me3\MMV\me3\mod
    chr / action / msg   tools\relic_assets.ps1（词条用到的动作事件、HKS、文本；动作与 HKS 取 MMV 或空城独立包的 mod\，文本取底包）
    sfx                  tools\relic_sfx.ps1（换色复制的自定义特效，加进底包的 commoneffects_dlc01；底包没有的特效包用原版）
    script / msg         tools\inherit_talk.ps1（锻造台的「继承词条」选项与列表文本，对话脚本先取 MMV 或空城独立包的 mod\，
                         没有的地图（第三天首领前集结点 m12 / m32）取原版）
    event\common_func    tools\merge_6p_common_func.ps1（只有 Mmv：MMV 与 6 人修复包的公共事件合并，修卡莉果入侵等卡死；
                         pkg_mmv 只给加载了 6 人修复包的无缝配置用，带空城的 pkg 与在线版共用，不能放）
    menu\01_common_h     tools\dlc_map_icons.ps1（只有 Mmv：湖沼 / 锻造村改用图标行 124 / 125 并换上 DLC 原图，
                         regulation 那一半由 mmv_relics --mmv-map-icons 同时改）
    三档锻体             只有 Mmv：mmv_relics --tiered-buffs（金 / 紫 / 蓝锻体池与角色专属锻体，见 relics\src\tiered），
                         tools\tiered_hks.ps1 再往 pkg 的 c0000.hks 插入触发脚本（tools\tiered_buffs.lua、tiered_roles.lua），
                         tools\tiered_tae.ps1 往 c0000.anibnd 加条件事件，tools\tiered_moveset.ps1 加执行者8 的招式表 a293，
                         tools\tiered_grip.ps1 往 c0000_a6x 加改过握刀位置的动画，tools\tiered_events.ps1 改 common_func 的监牢事件
  原版独立版见 tools\build_vanilla_pkg.ps1。
  首次运行把原来的 pkg\regulation.bin 备份为 regulation.before-rebalance.bin。
  整合包更新后重跑；生成后联机的人必须复制同一套文件（regulation 加密 IV 随机，各自生成哈希不同）
#>
param(
    [ValidateSet("MmvKongCheng", "Mmv", "KongCheng")] [string]$Target = "MmvKongCheng",
    # 新首领上限取原版同档首领的分位：0.75 中高，1 最高
    [double]$Quantile = 0.75,
    # 不做首领重平衡，保持整合包原有的敌人强度
    [switch]$NoRebalance,
    # 输出目录，默认见上；游戏运行时可先生成到别处检查
    [string]$PkgDir = ""
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\.."
$game = Resolve-Path "$root\.."
$me3 = Join-Path $game "me3"
$mmvMod = Join-Path $me3 "MMV\me3\mod"
# Base：regulation / 文本 / 特效的底包；Assets：动作、HKS、对话脚本的来源
$preset = @{
    MmvKongCheng = @{ Base = Join-Path $me3 "MMV\me3\modKongCheng"; Assets = $mmvMod; Pkg = Join-Path $me3 "MMV\me3\StatAlloc\pkg" }
    Mmv          = @{ Base = $mmvMod; Assets = $mmvMod; Pkg = Join-Path $me3 "MMV\me3\StatAlloc\pkg_mmv"
                      SixPlayer = Join-Path $me3 "MMV\me3\NightreignPLUS\Nightreign6PlayerFixes.v1.1.1\event"; MapIcons = $true; TieredBuffs = $true }
    KongCheng    = @{ Base = Join-Path $me3 "kongcheng\mod"; Assets = Join-Path $me3 "kongcheng\mod"; Pkg = Join-Path $me3 "kongcheng\StatAlloc\pkg" }
}[$Target]
$baseDir = $preset.Base
$assets = $preset.Assets

. (Join-Path $PSScriptRoot "build_env.ps1")
Push-Location $root
try {
    # cargo 把进度写到 stderr，PowerShell 5.1 在 Stop 模式下会误判为错误
    $ErrorActionPreference = "Continue"
    cargo build --release -p mmv_rebalance -p mmv_relics -p nr_stat_alloc_patcher 2>&1 | ForEach-Object { "$_" } | Where-Object { $_ -notmatch "^\s+Compiling" }
    $code = $LASTEXITCODE
    $ErrorActionPreference = "Stop"
    if ($code -ne 0) { throw "cargo build failed ($code)" }
} finally { Pop-Location }

$bin = if ($env:CARGO_TARGET_DIR) { Join-Path $env:CARGO_TARGET_DIR "release" } else { Join-Path $root "target\release" }
# 转成绝对路径：读写 bnd 时会切换工作目录（oo2core），相对路径会写到 SoulsFormats 的依赖目录里
$pkg = if ($PkgDir) { $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($PkgDir) } else { $preset.Pkg }
$out = Join-Path $pkg "regulation.bin"
$backup = Join-Path $pkg "regulation.before-rebalance.bin"
$rebalanced = Join-Path $pkg "regulation.rebalanced.tmp"
$withRelics = Join-Path $pkg "regulation.relics.tmp"
New-Item -ItemType Directory -Force $pkg | Out-Null

function Invoke-Step([string]$Name, [scriptblock]$Block) {
    "== $Name"
    & $Block
    if ($LASTEXITCODE -ne 0) { throw "$Name failed ($LASTEXITCODE)" }
}
try {
    if ($NoRebalance) {
        Copy-Item (Join-Path $baseDir "regulation.bin") $rebalanced
    } else {
        Invoke-Step "mmv_rebalance" { & "$bin\mmv_rebalance.exe" (Join-Path $baseDir "regulation.bin") $rebalanced `
            --vanilla (Join-Path $game "Game\regulation.bin") --mmv (Join-Path $mmvMod "regulation.bin") --quantile $Quantile }
    }
    $relicArgs = @($rebalanced, $withRelics) + $(if ($preset.MapIcons) { @("--mmv-map-icons") } else { @() }) +
        $(if ($preset.TieredBuffs) { @("--tiered-buffs") } else { @() })
    Invoke-Step "mmv_relics" { & "$bin\mmv_relics.exe" @relicArgs }
    if ((Test-Path $out) -and -not (Test-Path $backup)) { Copy-Item $out $backup }
    Invoke-Step "nr_stat_alloc_patcher" { & "$bin\nr_stat_alloc_patcher.exe" $withRelics $out }
} finally {
    Remove-Item $rebalanced, $withRelics -ErrorAction SilentlyContinue
}
$vanillaDir = Join-Path $root "target\vanilla"
"== relic_assets"
& (Join-Path $PSScriptRoot "relic_assets.ps1") -AnibndSrc "$assets\chr\c0000.anibnd.dcx" -HksSrc "$assets\action\script\c0000.hks" `
    -MsgSrcDir "$baseDir\msg" -OutDir $pkg -TieredAbilities:([bool]$preset.TieredBuffs)
if ($preset.TieredBuffs) {
    "== tiered_hks"
    & (Join-Path $PSScriptRoot "tiered_hks.ps1") -Hks (Join-Path $pkg "action\script\c0000.hks")
    "== tiered_tae"
    & (Join-Path $PSScriptRoot "tiered_tae.ps1") -Anibnd (Join-Path $pkg "chr\c0000.anibnd.dcx")
    "== tiered_moveset"
    & (Join-Path $PSScriptRoot "tiered_moveset.ps1") -Anibnd (Join-Path $pkg "chr\c0000.anibnd.dcx")
    "== tiered_grip"
    & (Join-Path $PSScriptRoot "extract_vanilla.ps1") -OutDir $vanillaDir -Paths "/chr/c0000_a9x.anibnd.dcx"
    & (Join-Path $PSScriptRoot "tiered_grip.ps1") -A6xSrc "$assets\chr\c0000_a6x.anibnd.dcx" -A9xSrc "$vanillaDir\chr\c0000_a9x.anibnd.dcx" -OutDir $pkg
}
"== inherit_talk"
# 整合包没带的有锻造台的地图（第三天首领前集结点 m12 / m32）用原版
& (Join-Path $PSScriptRoot "extract_vanilla.ps1") -OutDir $vanillaDir -Paths "/script/talk/m12_00_00_00.talkesdbnd.dcx", "/script/talk/m32_00_00_00.talkesdbnd.dcx"
& (Join-Path $PSScriptRoot "inherit_talk.ps1") -TalkDirs "$assets\script\talk", "$vanillaDir\script\talk" -MsgSrcDir "$baseDir\msg" -OutDir $pkg
if ($preset.SixPlayer) {
    "== merge_6p_common_func"
    & (Join-Path $PSScriptRoot "extract_vanilla.ps1") -OutDir $vanillaDir -Paths "/event/common_func.emevd.dcx"
    & (Join-Path $PSScriptRoot "merge_6p_common_func.ps1") -MmvEvent "$assets\event" -SixPlayerEvent $preset.SixPlayer `
        -VanillaCommonFunc "$vanillaDir\event\common_func.emevd.dcx" -OutDir $pkg
    if ($preset.TieredBuffs) {
        "== tiered_events"
        & (Join-Path $PSScriptRoot "tiered_events.ps1") -Emevd (Join-Path $pkg "event\common_func.emevd.dcx")
    }
}
if ($preset.MapIcons) {
    "== dlc_map_icons"
    & (Join-Path $PSScriptRoot "dlc_map_icons.ps1") -MenuDir "$assets\menu" -OutDir $pkg
}
# 底包没改的特效包用原版（MMV 本体只改了 dlc01，必须以它为底，否则会盖掉 MMV 的特效）
$sfxSrc = @{}
foreach ($name in "sfxbnd_commoneffects", "sfxbnd_commoneffects_dlc01") {
    $path = Join-Path $baseDir "sfx\$name.ffxbnd.dcx"
    if (-not (Test-Path $path)) {
        "== extract_vanilla ($name)"
        & (Join-Path $PSScriptRoot "extract_vanilla.ps1") -OutDir (Join-Path $root "target\vanilla")
        $path = Join-Path $root "target\vanilla\sfx\$name.ffxbnd.dcx"
    }
    $sfxSrc[$name] = $path
}
"== relic_sfx"
& (Join-Path $PSScriptRoot "relic_sfx.ps1") -CommonSrc $sfxSrc["sfxbnd_commoneffects"] `
    -Dlc01Src $sfxSrc["sfxbnd_commoneffects_dlc01"] -OutDir $pkg
$pkgFull = (Get-Item $pkg).FullName
Get-ChildItem $pkgFull -Recurse -File | Where-Object { $_.Name -notlike "*.before-*" } | ForEach-Object {
    "{0}  {1,10}  {2}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.Substring(0, 8), $_.Length, $_.FullName.Substring($pkgFull.Length + 1)
}
