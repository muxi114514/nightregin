<#
  把 GoldenAge 武器用到的角色挂点加到 pkg 的 c0000.chrbnd.dcx（挂点表 WepAbsorpPosParam 按编号找 c0000.flver 上的假点）。
  阎魔刀：542 / 525 跟随左手武器骨骼，待机时刀和刀鞘都挂在左手；880 / 881 幻影剑发射点；513~519、523、524 阵式发射点；560 魔人外观。
  这些编号黑夜君临的 c0000 都没有。
  542 绕 L_Weapon 局部 Z 转 180°，配合 ga_moveset.ps1 翻转的 a073 动画：阎魔刀的动作外观不变，退回原版 a000 的动作也握对。
  525 不翻：a73 里导入 a000 动作的行（202115 等）靠它摆正，腰挂时也是 a000 动作。
  骨骼按名字对应（两边骨骼序号不同）。-CheckOnly 只做往返校验不写文件：读写一遍必须和原文件逐字节相同，否则不能用库来改。
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$ChrbndSrc = "$PSScriptRoot\..\..\me3\MMV\me3\mod\chr\c0000.chrbnd.dcx",
    [string]$GaRoot = "D:\steam\steamapps\common\ELDEN RING\Game\GA",
    [int[]]$Dummies = @(513, 514, 515, 516, 517, 518, 519, 523, 524, 525, 542, 560, 880, 881),
    [switch]$CheckOnly,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

$FlverSuffix = "\c0000.flver"
$GripFlipDummies = @(542)
$bnd = Read-Bnd (Resolve-Path $ChrbndSrc).Path
$file = Find-File $bnd $FlverSuffix
[byte[]]$original = $file.Bytes.ToArray()
$flver = [SoulsFormats.FLVER2]::Read($original)
[byte[]]$again = $flver.Write()
$same = $again.Length -eq $original.Length -and [Linq.Enumerable]::SequenceEqual($again, $original)
"c0000.flver 往返校验: $(if ($same) { '逐字节相同' } else { "不同（$($original.Length) -> $($again.Length)）" })，假点 $($flver.Dummies.Count)，骨骼 $($flver.Nodes.Count)"
if (-not $same) { throw "c0000.flver round trip differs; patch it another way" }

$ga = [SoulsFormats.FLVER2]::Read((Find-File (Read-Bnd "$GaRoot\chr\c0000.chrbnd.dcx") $FlverSuffix).Bytes)
function Get-NodeIndex($Target, $Source, [int]$Index) {
    if ($Index -lt 0) { return $Index }
    $name = $Source.Nodes[$Index].Name
    for ($i = 0; $i -lt $Target.Nodes.Count; $i++) { if ($Target.Nodes[$i].Name -eq $name) { return $i } }
    throw "bone $name not found in the target skeleton"
}
function Get-BindMatrix($Target, [int]$Index) {
    $m = [System.Numerics.Matrix4x4]::Identity
    while ($Index -ge 0) {
        $node = $Target.Nodes[$Index]
        $m = [System.Numerics.Matrix4x4]::Multiply($m, $node.ComputeLocalTransform())
        $Index = $node.ParentIndex
    }
    $m
}
function Get-Inverse($Matrix) {
    $inverse = [System.Numerics.Matrix4x4]::Identity
    if (-not [System.Numerics.Matrix4x4]::Invert($Matrix, [ref]$inverse)) { throw "matrix is not invertible" }
    $inverse
}
# 假点的坐标在父节点空间，转换到跟随骨骼的局部空间转 180° 再转回来
function Set-DummyFlippedZ($Target, $Dummy) {
    $toBone = [System.Numerics.Matrix4x4]::Multiply((Get-BindMatrix $Target $Dummy.ParentBoneIndex), (Get-Inverse (Get-BindMatrix $Target $Dummy.AttachBoneIndex)))
    $turn = [System.Numerics.Matrix4x4]::CreateRotationZ([Math]::PI)
    $total = [System.Numerics.Matrix4x4]::Multiply([System.Numerics.Matrix4x4]::Multiply($toBone, $turn), (Get-Inverse $toBone))
    $Dummy.Position = [System.Numerics.Vector3]::Transform($Dummy.Position, $total)
    $Dummy.Forward = [System.Numerics.Vector3]::TransformNormal($Dummy.Forward, $total)
    $Dummy.Upward = [System.Numerics.Vector3]::TransformNormal($Dummy.Upward, $total)
}
foreach ($id in $Dummies) {
    if ($flver.Dummies | Where-Object { $_.ReferenceID -eq $id }) { throw "dummy $id already exists" }
    $src = @($ga.Dummies | Where-Object { $_.ReferenceID -eq $id })
    if (-not $src.Count) { throw "dummy $id not found in GA" }
    foreach ($s in $src) {
        $d = New-Object SoulsFormats.FLVER+Dummy
        $d.Position = $s.Position
        $d.Forward = $s.Forward
        $d.Upward = $s.Upward
        $d.ReferenceID = $s.ReferenceID
        $d.Color = $s.Color
        $d.Flag1 = $s.Flag1
        $d.UseUpwardVector = $s.UseUpwardVector
        $d.Unk30 = $s.Unk30
        $d.Unk34 = $s.Unk34
        $d.ParentBoneIndex = [int16](Get-NodeIndex $flver $ga $s.ParentBoneIndex)
        $d.AttachBoneIndex = [int16](Get-NodeIndex $flver $ga $s.AttachBoneIndex)
        if ($GripFlipDummies -contains $id) { Set-DummyFlippedZ $flver $d }
        $flver.Dummies.Add($d)
        "假点 $id：跟随 $($flver.Nodes[$d.AttachBoneIndex].Name)，父 $($flver.Nodes[$d.ParentBoneIndex].Name)，位置 $($d.Position)"
    }
}
if ($CheckOnly) { "只校验，没有写文件"; return }
$file.Bytes = [byte[]]$flver.Write()
Write-Bnd $bnd "$((Resolve-Path $Pkg).Path)\chr\c0000.chrbnd.dcx"
$check = [SoulsFormats.FLVER2]::Read((Find-File (Read-Bnd "$((Resolve-Path $Pkg).Path)\chr\c0000.chrbnd.dcx") $FlverSuffix).Bytes)
foreach ($id in $Dummies) { if (-not ($check.Dummies | Where-Object { $_.ReferenceID -eq $id })) { throw "self-check: dummy $id missing" } }
"c0000.chrbnd：假点 $($flver.Dummies.Count - @($Dummies).Count) -> $($check.Dummies.Count)"
