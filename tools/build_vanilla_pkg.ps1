<#
  生成原版独立版的 pkg（tools\package.ps1 打包时原样带上，独立包与无缝包共用同一份）：
    regulation.bin       Game\regulation.bin → mmv_relics（自定义遗物词条、护符、追加攻击入口）→ nr_stat_alloc_patcher（属性组件）
    chr / action / msg   tools\relic_assets.ps1（以原版文件为底）
    sfx                  tools\relic_sfx.ps1（加进原版 commoneffects_dlc01）
    script / msg         tools\inherit_talk.ps1（锻造台的「继承词条」选项与列表文本）
  原版文件由 tools\extract_vanilla.ps1 从加密归档导出到 target\vanilla（已有则跳过）。没有首领重平衡（那是给空城包调的）。
  生成后分发的人必须复制同一套文件（regulation 加密 IV 随机，各自生成哈希不同）。
#>
param(
    [string]$PkgDir = "$PSScriptRoot\..\target\pkg_vanilla"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\.."
$game = Resolve-Path "$root\.."
$vanilla = Join-Path $root "target\vanilla"

. (Join-Path $PSScriptRoot "build_env.ps1")
Push-Location $root
try {
    # cargo 把进度写到 stderr，PowerShell 5.1 在 Stop 模式下会误判为错误
    $ErrorActionPreference = "Continue"
    cargo build --release -p mmv_relics -p nr_stat_alloc_patcher 2>&1 | ForEach-Object { "$_" } | Where-Object { $_ -notmatch "^\s+Compiling" }
    $code = $LASTEXITCODE
    $ErrorActionPreference = "Stop"
    if ($code -ne 0) { throw "cargo build failed ($code)" }
} finally { Pop-Location }

"== extract_vanilla"
& (Join-Path $PSScriptRoot "extract_vanilla.ps1") -OutDir $vanilla

$bin = if ($env:CARGO_TARGET_DIR) { Join-Path $env:CARGO_TARGET_DIR "release" } else { Join-Path $root "target\release" }
if (Test-Path $PkgDir) { Remove-Item $PkgDir -Recurse -Force }
New-Item -ItemType Directory -Force $PkgDir | Out-Null
$PkgDir = (Resolve-Path $PkgDir).Path
$withRelics = Join-Path $PkgDir "regulation.relics.tmp"
try {
    "== mmv_relics"
    & "$bin\mmv_relics.exe" (Join-Path $game "Game\regulation.bin") $withRelics
    if ($LASTEXITCODE -ne 0) { throw "mmv_relics failed ($LASTEXITCODE)" }
    "== nr_stat_alloc_patcher"
    & "$bin\nr_stat_alloc_patcher.exe" $withRelics (Join-Path $PkgDir "regulation.bin")
    if ($LASTEXITCODE -ne 0) { throw "nr_stat_alloc_patcher failed ($LASTEXITCODE)" }
} finally {
    Remove-Item $withRelics -ErrorAction SilentlyContinue
}
"== relic_assets"
& (Join-Path $PSScriptRoot "relic_assets.ps1") -AnibndSrc "$vanilla\chr\c0000.anibnd.dcx" -HksSrc "$vanilla\action\script\c0000.hks" `
    -MsgSrcDir "$vanilla\msg" -OutDir $PkgDir
"== inherit_talk"
& (Join-Path $PSScriptRoot "inherit_talk.ps1") -TalkDirs "$vanilla\script\talk" -MsgSrcDir "$vanilla\msg" -OutDir $PkgDir
"== relic_sfx"
& (Join-Path $PSScriptRoot "relic_sfx.ps1") -CommonSrc "$vanilla\sfx\sfxbnd_commoneffects.ffxbnd.dcx" `
    -Dlc01Src "$vanilla\sfx\sfxbnd_commoneffects_dlc01.ffxbnd.dcx" -OutDir $PkgDir
Get-ChildItem $PkgDir -Recurse -File | ForEach-Object {
    "{0}  {1,10}  {2}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.Substring(0, 8), $_.Length, $_.FullName.Substring($PkgDir.Length + 1)
}
