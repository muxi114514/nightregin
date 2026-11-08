<#
  把 GoldenAge 武器的招式表搬进 pkg（跟在 ga_port.ps1 之后运行）：
    chr\c0000.anibnd.dcx      加招式表 TAE（两边格式相同，文件编号 / 内部 ID 的规则也一样；除 $HostedMoves 外内容照拷）
    chr\c0000_a6x.anibnd.dcx  加招式表自己的动画。mod 的动画有两种：引用 compendium 的，编号要和 a6x 的相同；
                              自带类型信息的不依赖 compendium。两种都放进 a6x，不用多发一个分包
    material\allmaterial      补模型用到、黑夜君临没有的材质
  a073 的动画要翻 L_Weapon，配合 ga_chr_dummy.ps1 翻转的假点 542（见 $GripFlip）。
  -AnibndSrc / -A6xSrc 取 pkg 里已有的文件就地改。重复运行会报错（招式表已存在）
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$GaRoot = "D:\steam\steamapps\common\ELDEN RING\Game\GA",
    [string]$MaterialSrc = "$PSScriptRoot\..\..\me3\MMV\me3\mod\material\allmaterial.matbinbnd.dcx",
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

# 阎魔刀：spAtkcategory 73 → a73，手持 73 → a373。动画文件名的招式表号补零到三位
$TaeNumbers = 73, 373
$TaeBinderBase = 5000000
$TaeIdBase = 2000
$TaeTemplate = "\a20.tae"
# 行为图没有法环的潜行攻击状态，这些动作搬到招式表没用的现成状态上（招式表号 → 目标动作号 = 来源动作号），动画文件靠 TAE 导入
$HostedMoves = @{ 73 = @{ 30700 = 32310; 30950 = 34310 } }
$ImportIdScale = 1000000
$GaAnimPacks = "c0000_dlc01", "c0000_dlc02"
$HkxPattern = "\\a0?73_\d+\.hkx$|\\a373_\d+\.hkx$"
# 阎魔刀的 a073 动作都把 L_Weapon（骨骼 78）转了 180° 才让刀鞘朝后，退回原版 a000 的动作没有这个翻转就握反了。
# 把翻转挪到假点 542 上（ga_chr_dummy.ps1），这里抵消掉：a073 的外观不变，a000 的动作也握对。a373 是待机姿势的叠加，不翻
$GripFlip = @{ Pattern = "\\a0?73_\d+\.hkx$"; Bone = 78 }
$Materials = @("s88302_Glow")
$NrInterRoot = "W:\CL\data\Target\INTERROOT_win64"
$GaInterRoot = "N:\GR\data\INTERROOT_win64"

function Add-Sorted($Bnd, $File) {
    if ($Bnd.Files | Where-Object { $_.ID -eq $File.ID }) { throw "file id $($File.ID) already used ($($File.Name))" }
    $at = 0
    while ($at -lt $Bnd.Files.Count -and $Bnd.Files[$at].ID -lt $File.ID) { $at++ }
    $Bnd.Files.Insert($at, $File)
}
function New-BinderFile($Template, [int64]$Id, [string]$Name, [byte[]]$Bytes) {
    $file = New-Object SoulsFormats.BinderFile
    $file.Flags = $Template.Flags
    $file.CompressionType = $Template.CompressionType
    $file.ID = $Id
    $file.Name = $Name
    $file.Bytes = $Bytes
    $file
}
function Move-TaeAnim($Tae, [int]$Number, [long]$Target, [long]$Source) {
    $to = Get-Anim $Tae $Target
    $from = Get-Anim $Tae $Source
    if (@($to.Events).Count -gt 0) { throw "a${Number}_$Target is in use" }
    $events = $to.Events; $groups = $to.EventGroups
    $to.Events = $from.Events; $to.EventGroups = $from.EventGroups
    $from.Events = $events; $from.EventGroups = $groups
    $to.AnimFileName = $from.AnimFileName
    $to.MiniHeader.ImportsHKX = $true
    $to.MiniHeader.ImportHKXSourceAnimID = [int]($Number * $ImportIdScale + $Source)
}
# Havok 标记文件里引用 compendium 的段
function Get-CompendiumRef([byte[]]$Bytes) {
    $i = [Text.Encoding]::ASCII.GetString($Bytes).IndexOf("TCRF", [StringComparison]::Ordinal)
    if ($i -lt 0) { return "" }
    [BitConverter]::ToString($Bytes, $i + 4, 8) -replace '-', ''
}
function Get-GripFlipped([byte[]]$Bytes, [string]$Name) {
    $tmp = [IO.Path]::GetTempFileName()
    $in = "$tmp.in.hkx"; $out = "$tmp.out.hkx"
    try {
        [IO.File]::WriteAllBytes($in, $Bytes)
        & python -I (Join-Path $PSScriptRoot "hkx_flip_bone.py") $in $out $GripFlip.Bone | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "hkx_flip_bone failed on $Name" }
        [IO.File]::ReadAllBytes($out)
    } finally { Remove-Item $tmp, $in, $out -ErrorAction SilentlyContinue }
}

$pkgPath = (Resolve-Path $Pkg).Path
$anibndPath = "$pkgPath\chr\c0000.anibnd.dcx"
$a6xPath = "$pkgPath\chr\c0000_a6x.anibnd.dcx"

# ---- 招式表 ----
$gaAni = Read-Bnd "$GaRoot\chr\c0000.anibnd.dcx"
$anibnd = Read-Bnd $anibndPath
$template = Find-File $anibnd $TaeTemplate
foreach ($n in $TaeNumbers) {
    $src = Find-File $gaAni "\a$n.tae"
    $tae = [SoulsFormats.TAE]::Read($src.Bytes)
    if ($tae.ID -ne $TaeIdBase + $n) { throw "a$n.tae: unexpected id $($tae.ID)" }
    $name = $template.Name -replace ([regex]::Escape($TaeTemplate.Substring(1)) + '$'), "a$n.tae"
    [byte[]]$taeBytes = $src.Bytes.ToArray()
    if ($HostedMoves.ContainsKey($n)) {
        foreach ($target in $HostedMoves[$n].Keys) { Move-TaeAnim $tae $n $target $HostedMoves[$n][$target] }
        $taeBytes = [byte[]]$tae.Write()
    }
    Add-Sorted $anibnd (New-BinderFile $template ($TaeBinderBase + $n) $name $taeBytes)
    "招式表 a$n：$($tae.Animations.Count) 段动作"
}
Write-Bnd $anibnd $anibndPath

# ---- 动画 ----
$a6x = Read-Bnd $a6xPath
$hkxTemplate = $a6x.Files | Where-Object { $_.Name.EndsWith(".hkx") } | Select-Object -First 1
$folder = [IO.Path]::GetDirectoryName($hkxTemplate.Name)
$a6xRef = Get-CompendiumRef $hkxTemplate.Bytes.ToArray()
$selfContained = 0; $viaCompendium = 0; $gripFlipped = 0
foreach ($p in $GaAnimPacks) {
    $bnd = Read-Bnd "$GaRoot\chr\$p.anibnd.dcx"
    foreach ($f in @($bnd.Files | Where-Object { $_.Name -match $HkxPattern })) {
        [byte[]]$bytes = $f.Bytes.ToArray()
        $ref = Get-CompendiumRef $bytes
        if ($ref -eq "") { $selfContained++ } elseif ($ref -eq $a6xRef) { $viaCompendium++ } else { throw "$($f.Name): compendium $ref differs from a6x $a6xRef" }
        if ($f.Name -match $GripFlip.Pattern) { $bytes = Get-GripFlipped $bytes $f.Name; $gripFlipped++ }
        Add-Sorted $a6x (New-BinderFile $hkxTemplate $f.ID "$folder\$([IO.Path]::GetFileName($f.Name))" $bytes)
    }
    $bnd = $null
    [GC]::Collect()
}
Write-Bnd $a6x $a6xPath
"c0000_a6x：加入动画 $($selfContained + $viaCompendium) 段（自带类型 $selfContained，同 compendium $viaCompendium，翻 L_Weapon $gripFlipped），共 $($a6x.Files.Count) 个文件"

# ---- 材质 ----
$mat = Read-Bnd (Resolve-Path $MaterialSrc).Path
$gaMats = @()
foreach ($n in "allmaterial_dlc01", "allmaterial") { $gaMats += @((Read-Bnd "$GaRoot\material\$n.matbinbnd.dcx").Files) }
$nextId = [int64]($mat.Files | ForEach-Object { $_.ID } | Measure-Object -Maximum).Maximum + 1
foreach ($m in $Materials) {
    if ($mat.Files | Where-Object { [IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $m }) { throw "material $m already exists" }
    $src = $gaMats | Where-Object { [IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $m } | Select-Object -First 1
    if (-not $src) { throw "material $m not found in GA" }
    $name = $src.Name.Replace($GaInterRoot, $NrInterRoot)
    $mat.Files.Add((New-BinderFile $mat.Files[0] $nextId $name $src.Bytes.ToArray()))
    $nextId++
}
Write-Bnd $mat "$pkgPath\material\allmaterial.matbinbnd.dcx"
"allmaterial：补材质 $($Materials -join ', ')"

# ---- 自检 ----
$check = Read-Bnd $anibndPath
foreach ($n in $TaeNumbers) { if (([SoulsFormats.TAE]::Read((Find-File $check "\a$n.tae").Bytes)).ID -ne $TaeIdBase + $n) { throw "self-check: a$n.tae" } }
$check = Read-Bnd $a6xPath
if (@($check.Files | Where-Object { $_.Name -match $HkxPattern }).Count -ne $selfContained + $viaCompendium) { throw "self-check: a6x animations" }
"ga_moveset 完成"
