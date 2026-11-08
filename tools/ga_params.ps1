<#
  ga_port.ps1 用的参数表读写函数（点引用，用它的 $NamePrefix、$ModeSample）。
#>
function Open-Param($Bnd, [string]$Table, [string]$DefPath) {
    $file = $Bnd.Files | Where-Object { $_.Name -match "\\$Table\.param$" } | Select-Object -First 1
    if (-not $file) { throw "$Table not found" }
    $param = [SoulsFormats.PARAM]::Read($file.Bytes)
    $def = [SoulsFormats.PARAMDEF]::XmlDeserialize($DefPath, $false)
    # 定义可能比实际行短（Smithbox 的黑夜君临 Bullet 少 16 字节），写回时尾部会被截掉，补占位字段保住
    $missing = $param.DetectedSize - $def.GetRowSize()
    if ($missing -gt 0) {
        $tail = New-Object SoulsFormats.PARAMDEF+Field($def, [SoulsFormats.PARAMDEF+DefType]::dummy8, "tailPadding")
        $tail.ArrayLength = $missing
        $def.Fields.Add($tail)
    }
    $param.ApplyParamdef($def)
    @{ File = $file; Param = $param; Rows = @{} }
}
function Get-Row($Table, [int]$Id) {
    if ($Table.Rows.Count -eq 0) { foreach ($r in $Table.Param.Rows) { if (-not $Table.Rows.ContainsKey($r.ID)) { $Table.Rows[$r.ID] = $r } } }
    $Table.Rows[$Id]
}
function Add-Row($Table, $Template, [int]$Id, [string]$Name) {
    if (Get-Row $Table $Id) { throw "$($Table.Param.ParamType) $Id already exists (already ported?)" }
    $row = New-Object SoulsFormats.PARAM+Row($Template)
    $row.ID = $Id
    $row.Name = "$NamePrefix$Name"
    $rows = $Table.Param.Rows
    $at = 0
    while ($at -lt $rows.Count -and $rows[$at].ID -lt $Id) { $at++ }
    $rows.Insert($at, $row)
    $Table.Rows[$Id] = $row
    $row
}
function Set-Cell($Row, [string]$Field, $Value) {
    $cell = $Row[$Field]
    if (-not $cell) { throw "field $Field not found" }
    $cell.Value = [Convert]::ChangeType($Value, $cell.Value.GetType())
}
# 按字段名复制（两边都有且类型能转的），Only 不为空时只复制列出的；返回复制数
function Copy-Fields($From, $To, [string[]]$Only = @(), [string[]]$Skip = @()) {
    $n = 0
    foreach ($c in $From.Cells) {
        $name = $c.Def.InternalName
        if ($name -match 'pad|reserve|dummy') { continue }
        if ($Only.Count -and $Only -notcontains $name) { continue }
        if ($Skip -contains $name) { continue }
        $dst = $To[$name]
        if (-not $dst) { continue }
        try { $dst.Value = [Convert]::ChangeType($c.Value, $dst.Value.GetType()) } catch { throw "$name = $($c.Value) does not fit $($dst.Value.GetType().Name)" }
        $n++
    }
    $n
}
# 两边「无变化」的取值不一定相同（如 physicsDiffence 法环 0、黑夜君临 1），所以不能整行照抄：
# 每个字段的中性值取抽样行里最常见的值。新行先全部重置成黑夜君临的中性值，再只写 GA 偏离法环中性值的字段。
# 但「等于法环众数」不等于中性：dmgLevel、子弹 life、effectTargetEnemy 都是实际数值，两边众数不同就会被改错，
# 所以用 Always 指定哪些字段总是照写：攻击、子弹、外观整行照写（$CopyAll），效果只有 $EffectAlways 之外才按中性值处理
function Get-Modes($Table) {
    if ($Table.Modes) { return $Table.Modes }
    $rows = $Table.Param.Rows
    $step = [Math]::Max(1, [int]($rows.Count / $ModeSample))
    $counts = @{}
    for ($i = 0; $i -lt $rows.Count; $i += $step) {
        foreach ($c in $rows[$i].Cells) {
            $n = $c.Def.InternalName
            if ($n -match 'pad|reserve|dummy') { continue }
            if (-not $counts.ContainsKey($n)) { $counts[$n] = @{} }
            $v = "$($c.Value)"
            $counts[$n][$v] = 1 + [int]$counts[$n][$v]
        }
    }
    $modes = @{}
    foreach ($n in $counts.Keys) { $modes[$n] = ($counts[$n].GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 1).Key }
    $Table.Modes = $modes
    $modes
}
# 返回 @{ Copied = 写入数; Dropped = 黑夜君临没有、但 GA 有实际取值的字段 }
function Copy-Delta($Src, $Dst, $SrcTable, $DstTable, [hashtable]$Alias = @{}, [string[]]$Skip = @(), [string]$Always = "") {
    $srcModes = Get-Modes $SrcTable
    $dstModes = Get-Modes $DstTable
    foreach ($c in $Dst.Cells) {
        $n = $c.Def.InternalName
        if ($dstModes.ContainsKey($n)) { $c.Value = [Convert]::ChangeType($dstModes[$n], $c.Value.GetType()) }
    }
    $copied = 0; $dropped = @()
    foreach ($c in $Src.Cells) {
        $n = $c.Def.InternalName
        if ($n -match 'pad|reserve|dummy' -or $Skip -contains $n) { continue }
        if ("$($c.Value)" -eq $srcModes[$n] -and -not ($Always -and $n -match $Always)) { continue }
        $d = $Dst[$n]
        if (-not $d) {
            if ($Alias.ContainsKey($n)) {
                $target = $Dst[$Alias[$n]]
                $target.Value = [Convert]::ChangeType([double]$target.Value * [double]$c.Value, $target.Value.GetType())
                $copied++
            } else { $dropped += "$n=$($c.Value)" }
            continue
        }
        try { $d.Value = [Convert]::ChangeType($c.Value, $d.Value.GetType()) } catch { throw "$n = $($c.Value) does not fit $($d.Value.GetType().Name)" }
        $copied++
    }
    @{ Copied = $copied; Dropped = $dropped }
}
# 掉落类别 6 按这张表抽词条（类别 2 是普通物品，不读它，掉出来没有词条）。行号 = 武器号，沿用模板的词条表，战技不随机（用武器自带的）
function Add-CustomWeapon($Table, [int]$Weapon, [int]$Template, [int]$AttachTable = 0) {
    $row = Add-Row $Table (Get-Row $Table $Template) $Weapon "Weapon"
    Set-Cell $row "targetWeaponId" $Weapon
    Set-Cell $row "swordArtsTableId" -1
    if ($AttachTable) { Set-Cell $row "attachEffectTableId_1" $AttachTable }
    $row
}
# 救援值 = 武器 reviveDamage × 攻击行 reviveDamageCorrection / 100，法环没有这个字段，带过来的攻击行都是 0。
# 原版武器招式的修正等于物理动作值；固定伤害的子弹动作值是 0，原版同类子弹约为固定伤害的 0.4 倍
$FlatReviveRatio = 0.4
function Set-ReviveCorrection($Row) {
    $motion = [int]$Row["atkPhysCorrection"].Value
    $flat = 0
    foreach ($f in "atkPhys", "atkMag", "atkFire", "atkThun", "atkDark") { $flat += [int]$Row[$f].Value }
    Set-Cell $Row "reviveDamageCorrection" $(if ($motion -gt 0) { $motion } else { [Math]::Round($flat * $FlatReviveRatio) })
}
# 模板里黑夜君临独有的文字 / 图标字段不能留着，否则显示成模板的内容
function Clear-EffectUi($Row) {
    foreach ($c in $Row.Cells) {
        if ($c.Def.InternalName -match '^(spEffectTextId_|iconId)') { $c.Value = [Convert]::ChangeType(-1, $c.Value.GetType()) }
    }
}
