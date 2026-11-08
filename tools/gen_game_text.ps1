<#
  从游戏文本包生成面板用的名称表：
    src\data\relic_text.rs：词条名（AttachEffectName，遗物与武器词条共用）、遗物名（AntiqueName）、
      遗物筛选分组名（CL_MenuText 的 60000~60999 武器类别、136050~136199 筛选分类、288050~288099 角色名）
    src\data\weapon_text.rs：武器名（WeaponName，词条继承面板与初始武器选择用）
  来源按顺序读取，后面的覆盖前面的：原版（extract_vanilla.ps1 导出）、独立版补丁包、空城 0.46 底包与补丁包、MMV 底包、MMV StatAlloc 补丁包
  （带空城的 pkg 与只有 MMV 的 pkg_mmv，后者才有新武器词条的文本）。
  空城排在 MMV 前：两边同 ID 的只有译名差异（武器 16 把、词条 85 条），以 MMV 为准，空城只补它独有的条目。
  只在开发机上运行；生成的文件随源码提交，DLL 运行时不读文本包。
#>
param(
    [string[]]$MsgDirs = @(
        "$PSScriptRoot\..\target\vanilla\msg",
        "$PSScriptRoot\..\target\pkg_vanilla\msg",
        "$PSScriptRoot\..\..\me3\kongcheng\mod\msg",
        "$PSScriptRoot\..\..\me3\kongcheng\StatAlloc\pkg\msg",
        "$PSScriptRoot\..\..\me3\MMV\me3\modKongCheng\msg",
        "$PSScriptRoot\..\..\me3\MMV\me3\StatAlloc\pkg\msg",
        "$PSScriptRoot\..\..\me3\MMV\me3\StatAlloc\pkg_mmv\msg"
    ),
    [string]$OutputDir = "$PSScriptRoot\..\src\data",
    [string]$SoulsFormatsDir = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$env:PATH = "$SoulsFormatsDir\x64;$SoulsFormatsDir\Dependencies;" + $env:PATH
$global:sfAppDir = "$SoulsFormatsDir\app"
[System.AppDomain]::CurrentDomain.add_AssemblyResolve({
    param($s, $e)
    $f = Join-Path $global:sfAppDir ((($e.Name -split ',')[0]) + ".dll")
    if (Test-Path $f) { return [System.Reflection.Assembly]::LoadFrom($f) }
    return $null
})
[void][System.Reflection.Assembly]::LoadFrom("$SoulsFormatsDir\app\Andre.SoulsFormats.dll")

# 表名 -> (输出文件, 文本包, FMG 名, 过滤)
$Tables = [ordered]@{
    EFFECT_NAMES = @("relic_text.rs", "item", @("AttachEffectName", "AttachEffectName_dlc01"), { param($id) $true })
    RELIC_NAMES  = @("relic_text.rs", "item", @("AntiqueName", "AntiqueName_dlc01"), { param($id) $true })
    GROUP_NAMES  = @("relic_text.rs", "menu", @("CL_MenuText", "CL_MenuText_dlc01"), { param($id) ($id -ge 60000 -and $id -le 60999) -or ($id -ge 136050 -and $id -le 136199) -or ($id -ge 288050 -and $id -le 288099) })
    WEAPON_NAMES = @("weapon_text.rs", "item", @("WeaponName", "WeaponName_dlc01"), { param($id) $true })
}
$Headers = @{
    "relic_text.rs"  = "遗物词条 / 遗物 / 筛选分组的中英文名称"
    "weapon_text.rs" = "武器的中英文名称（文本 ID 即 EquipParamWeapon 行号）"
}
$names = @{}
foreach ($t in $Tables.Keys) { $names[$t] = @{} }

function Read-Bnd([string]$path) {
    $prev = [IO.Directory]::GetCurrentDirectory()
    [IO.Directory]::SetCurrentDirectory("$SoulsFormatsDir\Dependencies")  # KRAK 解压需要 oo2core
    try { return [SoulsFormats.BND4]::Read($path) } finally { [IO.Directory]::SetCurrentDirectory($prev) }
}

foreach ($dir in $MsgDirs) {
    if (-not (Test-Path $dir)) { Write-Warning "skip missing $dir"; continue }
    foreach ($lang in @(@("zhocn", "zh"), @("engus", "en"))) {
        foreach ($t in $Tables.Keys) {
            $null, $kind, $fmgs, $filter = $Tables[$t]
            $path = Join-Path $dir "$($lang[0])\$($kind)_dlc01.msgbnd.dcx"
            if (-not (Test-Path $path)) { continue }
            $bnd = Read-Bnd (Resolve-Path $path).Path
            foreach ($fmgName in $fmgs) {
                $file = $bnd.Files | Where-Object { [IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $fmgName } | Select-Object -First 1
                if (-not $file) { continue }
                foreach ($e in ([SoulsFormats.FMG]::Read($file.Bytes)).Entries) {
                    if (-not $e.Text -or -not (& $filter $e.ID)) { continue }
                    if (-not $names[$t].ContainsKey($e.ID)) { $names[$t][$e.ID] = @{} }
                    $names[$t][$e.ID][$lang[1]] = $e.Text
                }
            }
        }
    }
}

function Quote([string]$s) {
    '"' + ($s -replace '\\', '\\' -replace '"', '\"' -replace "`r", '' -replace "`n", '\n') + '"'
}
foreach ($file in ($Tables.Keys | ForEach-Object { $Tables[$_][0] } | Select-Object -Unique)) {
    $sb = New-Object System.Text.StringBuilder
    [void]$sb.AppendLine("//! $($Headers[$file])，由 tools/gen_game_text.ps1 从游戏文本包生成，不要手改。")
    [void]$sb.AppendLine("//! 每项 (文本 ID, 中文, 英文)，按 ID 升序；只有一种语言的文本时另一种用它代替。")
    [void]$sb.AppendLine("")
    foreach ($t in ($Tables.Keys | Where-Object { $Tables[$_][0] -eq $file })) {
        [void]$sb.AppendLine("pub static ${t}: &[(i32, &str, &str)] = &[")
        foreach ($id in ($names[$t].Keys | Sort-Object)) {
            $n = $names[$t][$id]
            $zh = if ($n.zh) { $n.zh } else { $n.en }
            $en = if ($n.en) { $n.en } else { $n.zh }
            [void]$sb.AppendLine("    ($id, $(Quote $zh), $(Quote $en)),")
        }
        [void]$sb.AppendLine("];")
        [void]$sb.AppendLine("")
        "{0}: {1} entries" -f $t, $names[$t].Count
    }
    $out = Join-Path (Resolve-Path $OutputDir).Path $file
    [IO.File]::WriteAllText($out, $sb.ToString().TrimEnd() + "`n", (New-Object System.Text.UTF8Encoding $false))
    "written $out"
}
