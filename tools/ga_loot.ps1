<#
  移植物品的掉落：放进首领球第三选项（含徘徊赐福的 ItemLotParam_enemy / _map 行），和徘徊赐福同池。
  三件合计 95/1000（金档锻体的出率），权重从该行局内词条表（类别 7）按比例扣，徘徊赐福不动，总和不变。
  每行至少留一个空条目给 DLL 运行时加的锻造石【３】（shared/src/boss_loot.rs）。
  就地改 <Pkg>\regulation.bin（或 -Out 另存）。已加过的行会报错，不会叠加
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$Out = "",
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files",
    [string]$Defs = "D:\relink\114514\smithbox\Assets\PARAM\NR\Defs"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")
$NamePrefix = ""; $ModeSample = 1500
. (Join-Path $PSScriptRoot "ga_params.ps1")

# Category（掉落行的类别号）：4 护符、6 自定义武器（EquipParamCustomWeapon 行，掉落时抽词条）；2 是不带词条的普通武器，移植武器不要用
$Drops = @(
    @{ Name = "阎魔刀"; Category = 6; Id = 89000000; Weight = 32 },
    @{ Name = "八刻轮印"; Category = 4; Id = 20008; Weight = 32 },
    @{ Name = "世界之眼"; Category = 4; Id = 20015; Weight = 31 }
)
$WendingGrace = 700
$Goods = 1
$ItemTable = 7
$SlotCount = 8
$KeepFree = 1
# 新条目的其它字段照抄徘徊赐福那一格，幸运加成关掉（出率固定）
$SlotFields = "cumulateLotPoint", "getItemFlagId", "lotItemNum", "cumulateReset", "lotActionButtonTypeA_", "lotActionButtonTypeB_"

function Get-Slot($Row, [int]$i) {
    $n = "0$i"
    [pscustomobject]@{ Index = $i; Id =[int]$Row["lotItemId$n"].Value; Category = [int]$Row["lotItemCategory$n"].Value; Weight = [int]$Row["lotItemBasePoint$n"].Value }
}

# 单行：按比例从局内词条表扣 $take，余数扣最大的那张，每张至少留 1；返回改了几处
function Add-Drops($Row) {
    $slots = @(1..$SlotCount | ForEach-Object { Get-Slot $Row $_ })
    $grace = $slots | Where-Object { $_.Category -eq $Goods -and $_.Id -eq $WendingGrace } | Select-Object -First 1
    if (-not $grace) { return $false }
    foreach ($d in $Drops) {
        if ($slots | Where-Object { $_.Category -eq $d.Category -and $_.Id -eq $d.Id }) { throw "row $($Row.ID) already has $($d.Name)" }
    }
    $free = @($slots | Where-Object { $_.Id -le 0 -and $_.Weight -eq 0 })
    if ($free.Count -lt $Drops.Count + $KeepFree) { throw "row $($Row.ID): only $($free.Count) free slots" }
    $tables = @($slots | Where-Object { $_.Category -eq $ItemTable -and $_.Weight -gt 0 })
    $pool = ($tables | Measure-Object -Property Weight -Sum).Sum
    $take = ($Drops | ForEach-Object { $_.Weight } | Measure-Object -Sum).Sum
    if ($pool - $tables.Count -lt $take) { throw "row $($Row.ID): item tables only weigh $pool" }
    $before = ($slots | Measure-Object -Property Weight -Sum).Sum
    $taken = 0
    foreach ($t in $tables) {
        $cut = [Math]::Floor($take * $t.Weight / $pool)
        $t.Weight -= $cut; $taken += $cut
    }
    $largest = $tables | Sort-Object Weight -Descending | Select-Object -First 1
    $largest.Weight -= $take - $taken
    foreach ($t in $tables) { Set-Cell $Row "lotItemBasePoint0$($t.Index)" $t.Weight }
    $g = "0$($grace.Index)"
    for ($k = 0; $k -lt $Drops.Count; $k++) {
        $d = $Drops[$k]; $n = "0$($free[$k].Index)"
        foreach ($f in $SlotFields) { Set-Cell $Row "$f$n" $Row["$f$g"].Value }
        Set-Cell $Row "enableLuck$n" 0
        Set-Cell $Row "lotItemId$n" $d.Id
        Set-Cell $Row "lotItemCategory$n" $d.Category
        Set-Cell $Row "lotItemBasePoint$n" $d.Weight
    }
    $after = (1..$SlotCount | ForEach-Object { (Get-Slot $Row $_).Weight } | Measure-Object -Sum).Sum
    if ($after -ne $before) { throw "row $($Row.ID): weight total $before -> $after" }
    return $true
}

$regPath = (Resolve-Path "$Pkg\regulation.bin").Path
$outPath = if ($Out) { $Out } else { $regPath }
Invoke-InDeps { $script:bnd = [SoulsFormats.SFUtil]::DecryptNightreignRegulation($regPath) }
$total = 0
foreach ($name in "ItemLotParam_enemy", "ItemLotParam_map") {
    $t = Open-Param $bnd $name "$Defs\ItemLotParam.xml"
    $n = 0
    foreach ($row in $t.Param.Rows) { if (Add-Drops $row) { $n++ } }
    if ($n) {
        [byte[]]$bytes = $t.Param.Write()
        if ([SoulsFormats.PARAM]::Read($bytes).DetectedSize -ne $t.Param.DetectedSize) { throw "$name row size changed" }
        $t.File.Bytes = $bytes
    }
    "$name：$n 行加入 " + (($Drops | ForEach-Object { "$($_.Name) $($_.Weight)" }) -join "、")
    $total += $n
}
if ($total -eq 0) { throw "no row with Wending Grace found" }
Invoke-InDeps { [SoulsFormats.SFUtil]::EncryptNightreignRegulation($outPath, $bnd) }

# 自检：读回后每行都含三件物品、总权重 1000、仍有空位
Invoke-InDeps { $script:check = [SoulsFormats.SFUtil]::DecryptNightreignRegulation($outPath) }
$found = 0
foreach ($name in "ItemLotParam_enemy", "ItemLotParam_map") {
    $t = Open-Param $check $name "$Defs\ItemLotParam.xml"
    foreach ($row in $t.Param.Rows) {
        $slots = @(1..$SlotCount | ForEach-Object { Get-Slot $row $_ })
        if (-not ($slots | Where-Object { $_.Category -eq $Goods -and $_.Id -eq $WendingGrace })) { continue }
        $found++
        foreach ($d in $Drops) {
            if (-not ($slots | Where-Object { $_.Category -eq $d.Category -and $_.Id -eq $d.Id -and $_.Weight -eq $d.Weight })) { throw "self-check: row $($row.ID) misses $($d.Name)" }
        }
        if (@($slots | Where-Object { $_.Id -le 0 -and $_.Weight -eq 0 }).Count -lt $KeepFree) { throw "self-check: row $($row.ID) has no free slot" }
    }
}
if ($found -ne $total) { throw "self-check: $found rows, expected $total" }
"ga_loot 完成：$total 行，" + (Get-FileHash $outPath).Hash.Substring(0, 8) + " -> $outPath"
