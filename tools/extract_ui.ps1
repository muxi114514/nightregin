<#
  从整合包当前生效的菜单图集（01_common_h、21_common_dlc01_h）提取面板用的原生界面素材、全部武器图标与遗物图标，输出到 StatAlloc\skin\。
  步骤：SoulsFormats 解出图集 DDS 与子图布局 -> extract_ui.py（Pillow）按名字裁剪为 PNG。
  只在开发机运行一次即可；skin 文件夹缺失时 DLL 会退回无素材的样式。
#>
param(
    [string]$MenuDir = "$PSScriptRoot\..\..\me3\NightreignPLUS\More Map and Weapons\menu",
    # 整合包没改的菜单文件从这里取（如空城 0.46 没有 21_common_dlc01_h，用 extract_vanilla.ps1 导出到 target\vanilla\menu 的原版）
    [string]$FallbackMenuDir = "$PSScriptRoot\..\target\vanilla\menu",
    [string]$Output = "$PSScriptRoot\..\..\me3\NightreignPLUS\StatAlloc\skin",
    [string]$SoulsFormatsDir = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files",
    [string]$Python = "python"
)
$ErrorActionPreference = "Stop"
# 菜单文件 -> 要导出的图集（物品图标图集里有初始武器图标）
$Atlases = [ordered]@{
    "01_common_h"       = @("SB_Common_1", "SB_In_Game_0", "SB_In_Game_1", "SB_KeyTop_0", "SB_KeyTop_02_0", "SB_ItemIcon_0", "SB_ItemIcon_1", "SB_ItemIcon_2")
    "21_common_dlc01_h" = @("SB_ItemIcon_DLC01_0")
}
# 只有部分整合包才有的武器图标图集（MMV 新增武器），存在才导出；布局名与贴图名不同时按布局里的 imagePath 找贴图
$OptionalAtlases = @{
    "21_common_dlc01_h" = @("MoreWeapons", "MMV", "jem")
}

$env:PATH = "$SoulsFormatsDir\x64;$SoulsFormatsDir\Dependencies;" + $env:PATH
$global:sfAppDir = "$SoulsFormatsDir\app"
[System.AppDomain]::CurrentDomain.add_AssemblyResolve({
    param($s, $e)
    $f = Join-Path $global:sfAppDir ((($e.Name -split ',')[0]) + ".dll")
    if (Test-Path $f) { return [System.Reflection.Assembly]::LoadFrom($f) }
    return $null
})
[void][System.Reflection.Assembly]::LoadFrom("$SoulsFormatsDir\app\Andre.SoulsFormats.dll")

# 空城 0.46 的 01_common_h.sblytbnd 末尾哈希表没按 16 对齐，SoulsFormats 校验哈希表头失败；
# 提取素材用不到哈希表，失败时去掉它（+0x32 扩展标志、+0x38 哈希表偏移清零）再解析
function Read-Layouts([string]$Path) {
    try { return [SoulsFormats.BND4]::Read($Path) } catch {
        $raw = [SoulsFormats.DCX]::Decompress($Path).ToArray()
        $raw[0x32] = 0
        [Array]::Clear($raw, 0x38, 8)
        return [SoulsFormats.BND4]::Read($raw)
    }
}

$work = Join-Path ([IO.Path]::GetTempPath()) "nr_stat_alloc_skin"
New-Item -ItemType Directory -Force $work | Out-Null
foreach ($menu in $Atlases.Keys) {
    $dir = if (Test-Path "$MenuDir\$menu.tpf.dcx") { $MenuDir } else { $FallbackMenuDir }
    $tpfPath = (Resolve-Path "$dir\$menu.tpf.dcx").Path
    $layoutPath = (Resolve-Path "$dir\$menu.sblytbnd.dcx").Path
    $prevDir = [IO.Directory]::GetCurrentDirectory()
    [IO.Directory]::SetCurrentDirectory("$SoulsFormatsDir\Dependencies")  # 解压 DCX 需要 oo2core
    try {
        $tpf = [SoulsFormats.TPF]::Read($tpfPath)
        $layouts = Read-Layouts $layoutPath
    } finally { [IO.Directory]::SetCurrentDirectory($prevDir) }
    foreach ($name in @($Atlases[$menu]) + @($OptionalAtlases[$menu] | Where-Object { $_ })) {
        $lay = $layouts.Files | Where-Object { (Split-Path $_.Name -Leaf) -eq "$name.layout" } | Select-Object -First 1
        $texName = $name
        if ($lay -and [Text.Encoding]::UTF8.GetString($lay.Bytes.ToArray()) -match 'imagePath="([^"]+)"') { $texName = [IO.Path]::GetFileNameWithoutExtension($Matches[1]) }
        $tex = $tpf.Textures | Where-Object { $_.Name -eq $name -or $_.Name -eq $texName } | Select-Object -First 1
        if (-not $tex -or -not $lay) {
            if ($Atlases[$menu] -contains $name) { throw "$name missing in $menu" }
            continue
        }
        [IO.File]::WriteAllBytes("$work\$name.dds", $tex.Bytes)
        [IO.File]::WriteAllBytes("$work\$name.layout", $lay.Bytes.ToArray())
    }
}

New-Item -ItemType Directory -Force $Output | Out-Null
& $Python -I "$PSScriptRoot\extract_ui.py" $work (Resolve-Path $Output).Path
if ($LASTEXITCODE -ne 0) { throw "extract_ui.py failed ($LASTEXITCODE)" }
Remove-Item -Recurse -Force $work
"{0:N1} MB in {1}" -f ((Get-ChildItem $Output -File -Recurse | Measure-Object Length -Sum).Sum / 1MB), (Resolve-Path $Output).Path
