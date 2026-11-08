<#
  图鉴导出第二步：读整合包 regulation（pkg_mmv）里图鉴要用的参数表，导出成 JSON（只读，不写回）。
  另读原版、MMV 底包、1.9.4 发布包的 regulation，只取行号与行名，用来判断「MMV 新增」与「1.9.5 新增」。
  产物 <Out>\params.json：{ pkg: { 表: [ { ID, Name, 字段... } ] }, ids: { vanilla|mmv|rel194: { 表: [ID] } }, gaRows: { 表: [ { ID, Name } ] } }
#>
param(
    [string]$Pkg = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\MMV\me3\StatAlloc\pkg_mmv\regulation.bin",
    [hashtable]$Others = @{
        vanilla = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\Game\regulation.bin"
        mmv     = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\MMV\me3\mod\regulation.bin"
        rel194  = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3-max\NRStatAlloc-1.9.4-MMV\StatAlloc\pkg_mmv\regulation.bin"
    },
    [string]$Defs = "D:\relink\114514\smithbox\Assets\PARAM\NR\Defs",
    [string]$Erbm = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\NightreignPLUS\ERBMv1.1\Files",
    [Parameter(Mandatory)] [string]$Out
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\NRStatAlloc\tools\tae_helpers.ps1"
. "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\NRStatAlloc\tools\ga_params.ps1"
Add-Type -AssemblyName System.Web.Extensions
$D = [System.Collections.Generic.Dictionary[string,object]]
$L = [System.Collections.Generic.List[object]]

# 表 → 要导出的字段（空 = 全部非填充字段）
$Wanted = [ordered]@{
    EquipParamWeapon     = @("sortId", "weight", "iconId", "wepType", "rarity", "attackBasePhysics", "attackBaseMagic", "attackBaseFire", "attackBaseThunder", "attackBaseDark",
        "correctStrength", "correctAgility", "correctMagic", "correctFaith", "correctLuck", "properStrength", "properAgility", "properMagic", "properFaith", "properLuck",
        "swordArtsParamId", "reinforceTypeId", "wepmotionCategory", "spAttribute", "atkAttribute", "atkAttribute2", "spEffectMsgId0", "spEffectMsgId1", "spEffectMsgId2",
        "heroID", "heroMovesetOverrideID", "gemMountType", "disableParam_NT")
    EquipParamAccessory  = @()
    Magic                = @()
    SwordArtsParam       = @("textId", "iconId", "swordArtsType", "sortId")
    NightBossMenuParam   = @()
    RareMapInfoMenuParam = @()
    EquipParamGoods      = @("sortId", "iconId", "goodsType", "rarity", "refId_default")
}
$IdTables = "EquipParamWeapon", "EquipParamAccessory", "Magic", "NightBossMenuParam", "RareMapInfoMenuParam", "EquipParamGoods", "AttachEffectParam"

function Convert-Value($v) {
    if ($null -eq $v) { return $null }
    if ($v -is [byte[]]) { return [Convert]::ToBase64String($v) }
    if ($v -is [float] -or $v -is [double]) { return [double]$v }
    if ($v -is [bool]) { return [bool]$v }
    if ($v -is [string]) { return $v }
    return [long]$v
}
function Export-Rows($Table, [string[]]$Fields, $OnlyIds = $null) {
    $list = $L::new()
    $rows = $Table.Param.Rows
    if ($rows.Count -eq 0) { return $list }
    # 字段名 → 列号只算一次，逐格按名字找太慢
    $idx = [ordered]@{}
    $cells = $rows[0].Cells
    for ($i = 0; $i -lt $cells.Count; $i++) {
        $n = $cells[$i].Def.InternalName
        if ($Fields.Count) { if ($Fields -contains $n) { $idx[$n] = $i } }
        elseif ($n -notmatch 'pad|reserve|dummy|Padding') { $idx[$n] = $i }
    }
    foreach ($r in $rows) {
        if ($OnlyIds -and -not $OnlyIds.Contains([int]$r.ID)) { continue }
        $o = $D::new()
        $o["ID"] = [int]$r.ID
        $o["Name"] = "$($r.Name)"
        foreach ($n in $idx.Keys) { $o[$n] = Convert-Value $r.Cells[$idx[$n]].Value }
        $list.Add($o)
    }
    $list
}

$root = $D::new()
$root["pkg"] = $D::new()
$root["ids"] = $D::new()
$root["gaRows"] = $D::new()

$bnd = Invoke-InDeps { [SoulsFormats.SFUtil]::DecryptNightreignRegulation($Pkg) }
$attachIds = New-Object 'System.Collections.Generic.HashSet[int]'
foreach ($t in $Wanted.Keys) {
    $tab = Open-Param $bnd $t "$Defs\$t.xml"
    $root.pkg[$t] = Export-Rows $tab $Wanted[$t]
    "pkg {0}: {1} rows" -f $t, $root.pkg[$t].Count
    if ($t -eq "EquipParamAccessory") {
        foreach ($r in $root.pkg[$t]) { foreach ($s in "spEffectId_1", "spEffectId_2", "spEffectId_3") { if ($r[$s] -gt 0) { [void]$attachIds.Add([int]$r[$s]) } } }
    }
}
# 护符详情页三个槽填的是词条号，只导出被护符引用到的词条
$tab = Open-Param $bnd "AttachEffectParam" "$Defs\AttachEffectParam.xml"
$root.pkg["AttachEffectParam"] = Export-Rows $tab @() $attachIds
"pkg AttachEffectParam: {0} rows" -f $root.pkg["AttachEffectParam"].Count

# ga_port.ps1 给移植行起的名字都带 [GA] 前缀
foreach ($f in $bnd.Files) {
    if ($f.Name -notmatch '\\([^\\]+)\.param$') { continue }
    $name = $Matches[1]
    try { $p = [SoulsFormats.PARAM]::Read($f.Bytes) } catch { continue }
    $ga = $L::new()
    foreach ($r in $p.Rows) { if ("$($r.Name)".StartsWith("[GA]")) { $o = $D::new(); $o["ID"] = [int]$r.ID; $o["Name"] = "$($r.Name)"; $ga.Add($o) } }
    if ($ga.Count) { $root.gaRows[$name] = $ga }
}
"gaRows tables: " + ($root.gaRows.Keys -join ", ")
$bnd = $null; [GC]::Collect()

foreach ($k in $Others.Keys) {
    if (-not (Test-Path $Others[$k])) { Write-Warning "skip $k"; continue }
    $b = Invoke-InDeps { [SoulsFormats.SFUtil]::DecryptNightreignRegulation($Others[$k]) }
    $root.ids[$k] = $D::new()
    foreach ($t in $IdTables) {
        $f = $b.Files | Where-Object { $_.Name -match "\\$t\.param$" } | Select-Object -First 1
        if (-not $f) { continue }
        $p = [SoulsFormats.PARAM]::Read($f.Bytes)
        $ids = $L::new()
        foreach ($r in $p.Rows) { $ids.Add([int]$r.ID) }
        $root.ids[$k][$t] = $ids
    }
    "{0}: weapons {1}" -f $k, $root.ids[$k]["EquipParamWeapon"].Count
    $b = $null; [GC]::Collect()
}

$ser = New-Object System.Web.Script.Serialization.JavaScriptSerializer
$ser.MaxJsonLength = [int]::MaxValue
New-Item -ItemType Directory -Force $Out | Out-Null
[IO.File]::WriteAllText((Join-Path $Out "params.json"), $ser.Serialize($root), (New-Object System.Text.UTF8Encoding $false))
"written $Out\params.json"
