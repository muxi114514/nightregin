<#
  自定义特效：从 commoneffects 复制原版特效、换色存成新 ID，连同资源清单加进 commoneffects_dlc01，
  输出到 pkg\sfx\sfxbnd_commoneffects_dlc01.ffxbnd.dcx（MMV 版以空城包的两个特效包为底，独立版以原版的为底）：
    450827 命定之死附魔（灵火附魔 303122 换色）    450828 命定之死地火（死亡勾棒灵火地火 450822 换色）
  dlc01 的特效本来就大量引用 commoneffects 的贴图，新特效沿用原资源清单即可。
  换色只改带色相的颜色（颜色属性，以及动作字段里「整数后连续 3 个 0~1 浮点」的 RGB），中性的白色倍率与发光强度不动；
  灵火的「白度」（最小 / 最大通道）线性映射为黑刀战技 450274 命定之死火焰的配色：R 0.749，G = B 在 0.188~0.467 之间。
  黑夜君临 FXR 头部 +4 为 1（艾尔登法环为 0），其余结构同艾尔登法环 v5：读前清零、写后还原。
  空城包的 commoneffects 是 DFLT 压缩但参数特殊，ERBM 的 SoulsFormats 不认，直接用 .NET 的 Deflate 解压；原版是 KRAK，照常读。
#>
param(
    [Parameter(Mandatory)] [string]$CommonSrc,
    [Parameter(Mandatory)] [string]$Dlc01Src,
    [Parameter(Mandatory)] [string]$OutDir,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path

$Recolors = @(
    @{ Source = 303122; Target = 450827; Name = "命定之死附魔（灵火附魔换色）" },
    @{ Source = 450822; Target = 450828; Name = "命定之死地火（灵火地火换色）" }
)
# 命定之死配色：红色通道固定为 Red，绿 / 蓝随原色白度从 WhiteFrom~1 线性映射到 Low~High
$Palette = @{ Red = 0.749; Low = 0.188; High = 0.467; WhiteFrom = 0.6 }
# 带色相的判定：(最大 - 最小) / 最大 不小于它
$MinSaturation = 0.05

[AppDomain]::CurrentDomain.add_AssemblyResolve({ param($s, $e) $p = [IO.Path]::Combine($app, $e.Name.Split(',')[0] + '.dll'); if ([IO.File]::Exists($p)) { [Reflection.Assembly]::LoadFrom($p) } else { $null } }) | Out-Null
[Reflection.Assembly]::LoadFrom("$app\Andre.SoulsFormats.dll") | Out-Null

function Invoke-InDeps([scriptblock]$Block) {
    $old = [IO.Directory]::GetCurrentDirectory()
    [IO.Directory]::SetCurrentDirectory($deps)
    try { & $Block } finally { [IO.Directory]::SetCurrentDirectory($old) }
}

# 读特效包：DFLT 压缩的 DCX 头 0x4C 字节（+0x1C 大端为解压后大小），之后是 zlib 流（跳过 2 字节 zlib 头交给 Deflate）；
# 其他压缩交给 SoulsFormats
function Read-Bank([string]$Path) {
    $fs = [IO.File]::OpenRead($Path)
    try {
        $head = New-Object byte[] 0x4E
        [void]$fs.Read($head, 0, $head.Length)
        if ([Text.Encoding]::ASCII.GetString($head, 0x28, 4) -ne "DFLT") { $fs.Dispose(); return Invoke-InDeps { [SoulsFormats.BND4]::Read($Path) } }
        $size = ([int]$head[0x1C] -shl 24) -bor ([int]$head[0x1D] -shl 16) -bor ([int]$head[0x1E] -shl 8) -bor [int]$head[0x1F]
        $ms = New-Object IO.MemoryStream $size
        $z = New-Object IO.Compression.DeflateStream($fs, [IO.Compression.CompressionMode]::Decompress)
        $z.CopyTo($ms)
        if ($ms.Length -ne $size) { throw "$Path decompressed to $($ms.Length) bytes, expected $size" }
        [SoulsFormats.BND4]::Read($ms.ToArray())
    } finally { $fs.Dispose() }
}

# ---- 换色 ----
function Convert-Rgb([double]$r, [double]$g, [double]$b) {
    $max = [Math]::Max($r, [Math]::Max($g, $b)); $min = [Math]::Min($r, [Math]::Min($g, $b))
    if ($max -le 0 -or ($max - $min) / $max -lt $MinSaturation) { return $null }
    $t = [Math]::Min(1.0, [Math]::Max(0.0, ($min / $max - $Palette.WhiteFrom) / (1 - $Palette.WhiteFrom)))
    $gb = $Palette.Low + $t * ($Palette.High - $Palette.Low)
    @(($Palette.Red * $max), ($gb * $max), ($gb * $max))
}
# 把 Fields[$at..$at+2] 的 RGB 换色，返回是否改动
function Set-Rgb($fields, [int]$at) {
    $c = Convert-Rgb $fields[$at].Value $fields[$at + 1].Value $fields[$at + 2].Value
    if (-not $c) { return $false }
    for ($k = 0; $k -lt 3; $k++) { $fields[$at + $k].Value = [single]$c[$k] }
    $true
}
# 颜色属性：Constant = RGBA；Stepped / Linear = [关键帧数 N, 最小 RGBA, 最大 RGBA, N 个时间, N 个 RGBA]（最小 / 最大随之重算）
function Update-Property($p) {
    $n = 0
    if ("$($p.PropertyType)" -eq "Color") {
        $f = $p.Fields
        switch ("$($p.InterpolationType)") {
            "Constant" { if (Set-Rgb $f 0) { $n++ } }
            { $_ -in "Stepped", "Linear" } {
                $keys = $f[0].Value
                if ($f.Count -ne 9 + 5 * $keys) { throw "unexpected color layout ($($f.Count) fields, $keys keys)" }
                $first = 9 + $keys
                for ($k = 0; $k -lt $keys; $k++) { if (Set-Rgb $f ($first + 4 * $k)) { $n++ } }
                for ($ch = 0; $ch -lt 3; $ch++) {
                    $vals = @(0..($keys - 1) | ForEach-Object { $f[$first + 4 * $_ + $ch].Value })
                    $f[1 + $ch].Value = [single]($vals | Measure-Object -Minimum).Minimum
                    $f[5 + $ch].Value = [single]($vals | Measure-Object -Maximum).Maximum
                }
            }
            { $_ -in "Zero", "One" } { }
            # 逐通道曲线（UnkAc6）等格式不改：原版灵火里这几处都是中性白
            default { $script:skipped++ }
        }
    }
    foreach ($m in $p.Modifiers) { foreach ($q in $m.Properties) { $n += Update-Property $q } }
    $n
}
# 动作字段里的 RGB：整数之后连续 3 个 0~1 的浮点（原版灵火为 0.75 / 0.875 / 1，血焰为 1 / 0.25 / 0.25）
function Update-Fields($fields) {
    $n = 0
    for ($i = 1; $i + 2 -lt $fields.Count; $i++) {
        if ("$($fields[$i - 1].Type)" -ne "Int") { continue }
        $rgb = $fields[$i..($i + 2)]
        if (@($rgb | Where-Object { "$($_.Type)" -ne "Float" -or $_.Value -lt 0 -or $_.Value -gt 1 }).Count) { continue }
        if (Set-Rgb $fields $i) { $n++; $i += 2 }
    }
    $n
}
function Update-Actions($actions) {
    $n = 0
    foreach ($a in $actions) {
        foreach ($p in @($a.Properties1) + @($a.Properties2)) { if ($p) { $n += Update-Property $p } }
        $n += Update-Fields $a.Fields1
        $n += Update-Fields $a.Fields2
    }
    $n
}
function Update-Container($c) {
    $n = Update-Actions $c.Actions
    foreach ($e in $c.Effects) { $n += Update-Actions $e.Actions }
    foreach ($k in $c.Containers) { $n += Update-Container $k }
    $n
}
function Copy-Recolored([byte[]]$Source, [int]$TargetId) {
    $bytes = [byte[]]$Source.Clone()
    $flag = $bytes[4]; $bytes[4] = 0
    $fxr = [SoulsFormats.FXR3]::Read($bytes)
    $fxr.Id = $TargetId
    $script:skipped = 0
    $changed = Update-Container $fxr.RootContainer
    if ($changed -eq 0) { throw "f${TargetId}: no color changed" }
    $out = [byte[]]$fxr.Write()
    $again = [byte[]][SoulsFormats.FXR3]::Read($out).Write()
    if (-not [Linq.Enumerable]::SequenceEqual($out, $again)) { throw "f${TargetId}: FXR write is not stable" }
    $out[4] = $flag
    @{ Bytes = $out; Changed = $changed; Skipped = $script:skipped }
}

# ---- 源特效 ----
$source = Read-Bank (Resolve-Path $CommonSrc).Path
function Find-Effect($bnd, [string]$Kind, [int]$Id) {
    $suffix = if ($Kind -eq "fxr") { "\effect\f{0:D9}.fxr" -f $Id } else { "\ResourceList\f{0:D9}.ffxreslist" -f $Id }
    $bnd.Files | Where-Object { $_.Name.EndsWith($suffix, [StringComparison]::OrdinalIgnoreCase) } | Select-Object -First 1
}

# ---- 写进 dlc01 ----
$dlcOut = Join-Path $OutDir "sfx\sfxbnd_commoneffects_dlc01.ffxbnd.dcx"
$dlc = Read-Bank (Resolve-Path $Dlc01Src).Path
$fxrs = @($dlc.Files | Where-Object { $_.Name -like "*.fxr" })
$lists = @($dlc.Files | Where-Object { $_.Name -like "*.ffxreslist" })
$prefix = $fxrs[0].Name.Substring(0, $fxrs[0].Name.LastIndexOf("\effect\") + 1)
$nextIndex = ($fxrs | Measure-Object ID -Maximum).Maximum + 1
foreach ($r in $Recolors) {
    if (Find-Effect $dlc "fxr" $r.Target) { throw "f$($r.Target) already exists in commoneffects_dlc01" }
    $fxr = Find-Effect $source "fxr" $r.Source
    $list = Find-Effect $source "list" $r.Source
    if (-not $fxr -or -not $list) { throw "source effect f$($r.Source) not found in commoneffects" }
    $new = Copy-Recolored $fxr.Bytes.ToArray() $r.Target
    $listId = 400000 + $nextIndex
    if ($dlc.Files | Where-Object { $_.ID -eq $nextIndex -or $_.ID -eq $listId }) { throw "file id $nextIndex / $listId already used" }
    $fxrFile = New-Object SoulsFormats.BinderFile($fxr.Flags, ("{0}effect\f{1:D9}.fxr" -f $prefix, $r.Target), $new.Bytes)
    $fxrFile.ID = $nextIndex
    $listFile = New-Object SoulsFormats.BinderFile($list.Flags, ("{0}ResourceList\f{1:D9}.ffxreslist" -f $prefix, $r.Target), $list.Bytes.ToArray())
    $listFile.ID = $listId
    $dlc.Files.Insert($dlc.Files.IndexOf($fxrs[-1]) + 1, $fxrFile)
    $dlc.Files.Insert($dlc.Files.IndexOf($lists[-1]) + 1, $listFile)
    $fxrs = @($dlc.Files | Where-Object { $_.Name -like "*.fxr" }); $lists = @($dlc.Files | Where-Object { $_.Name -like "*.ffxreslist" })
    "  f$($r.Target) $($r.Name)：改色 $($new.Changed) 处（逐通道曲线 $($new.Skipped) 处未改），文件 $nextIndex / $listId"
    $nextIndex++
}
New-Item -ItemType Directory -Force (Split-Path $dlcOut) | Out-Null
Invoke-InDeps { $dlc.Write($dlcOut) }
"sfx\sfxbnd_commoneffects_dlc01.ffxbnd.dcx: 新增特效 $($Recolors.Count) 个（共 $($dlc.Files.Count) 个文件）"
