<#
  把法环格式的 FXR 转成黑夜君临格式（ga_sfx.ps1 点引用，要先加载 Andre.SoulsFormats）。对照两边同号特效得出三处差异：
    曲线属性  法环 Curve2：[N, 最小×c, 最大×c, 时间×N, 值×cN, 斜率×2cN]（c 个通道共用关键帧）
              黑夜君临 UnkAc6：[结束时间, N×c, 末值×c, 末值×c, 再逐通道：时间×N, 值×N, 斜率×2N]
              黑夜君临原版一个 Curve2 都没有，所以照原版的写法转
              头部两组值约 10% 的原版行是最小 / 最大而不是末值，规律不明，这里统一取末值；时间、值、斜率与原版逐值相同
    渲染动作  黑夜君临的 Fields2 尾部多几个字段（原版里 609 补 1、0，其余补 0）
    文件头    +4 法环 0、黑夜君临 1
  库读不了「状态时间」操作数带字段的状态条件（校验过严，文件本身没问题），读前临时改成外部值类型，写后改回。
#>
$FxrFlagOffset = 4
$FxrIdOffset = 12
# 文件头：状态条件表的偏移和数量；每条 96 字节，左右操作数类型在 +16 / +56，其字段数在类型之后 8 字节
$FxrCondTableAt = 32
$FxrCondSize = 96
$FxrOperandOffsets = 16, 56
$FxrOperandCountAfter = 8
$FxrOperandStateTime = 0xFF
$FxrOperandExternal = 0xFD
$FxrChannels = @{ Scalar = 1; Vector2 = 2; Vector3 = 3; Color = 4 }
$FxrFields2Size = @{ 603 = 45; 604 = 46; 605 = 39; 10012 = 42 }
$FxrPointLight = 609
$FxrPointLightOldSize = 31
# 贴图 / 模型编号的位置（原版统计，这些位置的值全部命中资源清单）；属性里是浮点常量，不是整数，小编号也是真贴图
$FxrResourceSlots = @{
    600 = @{ P1 = @(0) }
    603 = @{ P1 = @(0); F1 = @(1); F2 = @(33) }
    604 = @{ F1 = @(1, 2, 3); F2 = @(43) }
    605 = @{ P1 = @(0) }
    606 = @{ P1 = @(0) }
    607 = @{ F1 = @(4) }
    608 = @{ P1 = @(1) }
    10012 = @{ P1 = @(0); F1 = @(1) }
    10013 = @{ F1 = @(0) }
    10014 = @{ F1 = @(0, 1) }
    10015 = @{ P1 = @(0) }
}

function New-FxrInt([int]$Value) { New-Object SoulsFormats.FXR3+Field+FieldInt($Value) }
function New-FxrFloat([single]$Value) { New-Object SoulsFormats.FXR3+Field+FieldFloat($Value) }

# 把「状态时间带字段」的操作数改成外部值或改回；返回改过的位置（在文件里的偏移相对条件表）
function Get-FxrStateTimeOperands([byte[]]$Bytes) {
    $table = [BitConverter]::ToInt32($Bytes, $FxrCondTableAt); $count = [BitConverter]::ToInt32($Bytes, $FxrCondTableAt + 4)
    for ($i = 0; $i -lt $count; $i++) {
        foreach ($rel in $FxrOperandOffsets) {
            $o = $i * $FxrCondSize + $rel
            if ($Bytes[$table + $o] -eq $FxrOperandStateTime -and $Bytes[$table + $o + 1] -eq $FxrOperandStateTime -and [BitConverter]::ToInt32($Bytes, $table + $o + $FxrOperandCountAfter) -gt 0) { $o }
        }
    }
}
function Set-FxrOperands([byte[]]$Bytes, $Positions, [byte]$Value) {
    $table = [BitConverter]::ToInt32($Bytes, $FxrCondTableAt)
    foreach ($o in $Positions) { $Bytes[$table + $o] = $Value }
}
function Read-FxrLoose([byte[]]$Bytes) {
    $copy = [byte[]]$Bytes.Clone()
    $copy[$FxrFlagOffset] = 0
    $positions = @(Get-FxrStateTimeOperands $copy)
    Set-FxrOperands $copy $positions $FxrOperandExternal
    @{ Fxr = [SoulsFormats.FXR3]::Read($copy); Positions = $positions }
}

function Convert-FxrCurve($Property) {
    $channels = $FxrChannels["$($Property.PropertyType)"]
    $old = @($Property.Fields)
    $keys = [int]$old[0].Value
    $timesAt = 1 + 2 * $channels
    $valuesAt = $timesAt + $keys
    $slopesAt = $valuesAt + $keys * $channels
    if ($old.Count -ne $slopesAt + 2 * $keys * $channels) { throw "unexpected Curve2 layout: $($old.Count) fields, $keys keys, $channels channels" }
    $list = $Property.Fields
    $list.Clear()
    $list.Add((New-FxrFloat $old[$timesAt + $keys - 1].Value))
    for ($c = 0; $c -lt $channels; $c++) { $list.Add((New-FxrInt $keys)) }
    foreach ($twice in 0, 1) { for ($c = 0; $c -lt $channels; $c++) { $list.Add((New-FxrFloat $old[$valuesAt + ($keys - 1) * $channels + $c].Value)) } }
    for ($c = 0; $c -lt $channels; $c++) {
        for ($k = 0; $k -lt $keys; $k++) { $list.Add((New-FxrFloat $old[$timesAt + $k].Value)) }
        for ($k = 0; $k -lt $keys; $k++) { $list.Add((New-FxrFloat $old[$valuesAt + $k * $channels + $c].Value)) }
        for ($k = 0; $k -lt 2 * $keys; $k++) { $list.Add((New-FxrFloat $old[$slopesAt + $k * $channels + $c].Value)) }
    }
    $Property.InterpolationType = [SoulsFormats.FXR3+PropertyInterpolationType]::UnkAc6
}
function Update-FxrProperty($Property, $Stats) {
    if ("$($Property.InterpolationType)" -eq "Curve2") { Convert-FxrCurve $Property; $Stats.Curves++ }
    foreach ($m in $Property.Modifiers) { foreach ($q in $m.Properties) { if ($q) { Update-FxrProperty $q $Stats } } }
}
function Update-FxrAction($Action, $Stats) {
    foreach ($p in @($Action.Properties1) + @($Action.Properties2)) { if ($p) { Update-FxrProperty $p $Stats } }
    $fields = $Action.Fields2
    $id = [int]$Action.Id
    if ($id -eq $FxrPointLight) {
        if ($fields.Count -le $FxrPointLightOldSize) {
            while ($fields.Count -lt $FxrPointLightOldSize) { $fields.Add((New-FxrInt 0)); $Stats.Padded++ }
            $fields.Add((New-FxrInt 1)); $fields.Add((New-FxrInt 0)); $Stats.Padded += 2
        }
    } elseif ($FxrFields2Size.ContainsKey($id)) {
        while ($fields.Count -lt $FxrFields2Size[$id]) { $fields.Add((New-FxrInt 0)); $Stats.Padded++ }
    }
}
function Update-FxrContainer($Container, $Stats) {
    foreach ($a in $Container.Actions) { Update-FxrAction $a $Stats }
    foreach ($e in $Container.Effects) { foreach ($a in $e.Actions) { Update-FxrAction $a $Stats } }
    foreach ($k in $Container.Containers) { Update-FxrContainer $k $Stats }
}
function Add-FxrActionResources($Action, $Numbers) {
    $slots = $FxrResourceSlots[[int]$Action.Id]
    if (-not $slots) { return }
    foreach ($i in $slots.P1) {
        $p = @($Action.Properties1)[$i]
        if ($p -and "$($p.InterpolationType)" -eq "Constant") { $n = [int][double]@($p.Fields)[0].Value; if ($n -gt 0) { $Numbers[$n] = 1 } }
    }
    foreach ($pair in @(@($slots.F1, $Action.Fields1), @($slots.F2, $Action.Fields2))) {
        foreach ($i in $pair[0]) {
            $f = @($pair[1])[$i]
            if ($f -and "$($f.Type)" -eq "Int" -and [int]$f.Value -gt 0) { $Numbers[[int]$f.Value] = 1 }
        }
    }
}
function Add-FxrContainerResources($Container, $Numbers) {
    foreach ($a in $Container.Actions) { Add-FxrActionResources $a $Numbers }
    foreach ($e in $Container.Effects) { foreach ($a in $e.Actions) { Add-FxrActionResources $a $Numbers } }
    foreach ($k in $Container.Containers) { Add-FxrContainerResources $k $Numbers }
}
# 特效实际用到的贴图 / 模型编号
function Get-FxrResourceNumbers([byte[]]$Bytes) {
    $numbers = @{}
    Add-FxrContainerResources (Read-FxrLoose $Bytes).Fxr.RootContainer $numbers
    $numbers.Keys
}

# 返回 @{ Bytes; Curves; Padded }。$Id 写进文件头的内部号：GA 复制特效只改文件名，黑夜君临原版两者全部相同
function ConvertTo-NrFxr([byte[]]$Bytes, [int]$Id = 0) {
    $read = Read-FxrLoose $Bytes
    $stats = @{ Curves = 0; Padded = 0 }
    Update-FxrContainer $read.Fxr.RootContainer $stats
    [byte[]]$out = $read.Fxr.Write()
    Set-FxrOperands $out $read.Positions $FxrOperandStateTime
    # 写出后还要能再读，且没有漏网的 Curve2
    $check = Read-FxrLoose $out
    $again = @{ Curves = 0; Padded = 0 }
    Update-FxrContainer $check.Fxr.RootContainer $again
    if ($again.Curves -or $again.Padded) { throw "FXR conversion is not stable: $($again.Curves) curves, $($again.Padded) fields left" }
    $out[$FxrFlagOffset] = 1
    if ($Id) { [BitConverter]::GetBytes($Id).CopyTo($out, $FxrIdOffset) }
    @{ Bytes = $out; Curves = $stats.Curves; Padded = $stats.Padded }
}
