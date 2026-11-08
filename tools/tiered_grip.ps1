<#
  执行者8 的志留亚漩涡握刀（只有 MMV，配合 tiered_moveset.ps1 的 30500 / 30505）。
  志留亚的动画靠武器骨骼 R_Weapon 的位移换成握枪杆中段：起手与收招 Y ≈ 0.03，中段滑到 -0.45（X 也横移 -0.14）。妖刀短，手就握到了刀身上。
  位移是样条，量化下界正好是中段那个值，所以只改第一块的 X、Y 下界：中段落到手心前面（同欧赫剑舞起手时剑在手前的样子），
  起手与收招基本不动，过渡仍然平滑，文件大小不变。
  原版 a809 不动（志留亚本身还要用），改过的另存成 a293_030500 / 030505，放进 MMV 的 c0000_a6x 分包（两个分包的 compendium 相同）。
  -A6xSrc 取整合包的分包，-A9xSrc 取原版（tools\extract_vanilla.ps1），输出到 -OutDir\chr
#>
param(
    [Parameter(Mandatory)] [string]$A6xSrc,
    [Parameter(Mandatory)] [string]$A9xSrc,
    [Parameter(Mandatory)] [string]$OutDir,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
. (Join-Path $PSScriptRoot "tae_helpers.ps1")
. (Join-Path $PSScriptRoot "hkx_spline.ps1")

# Skeleton.hkx 里 R_Weapon 的骨骼号
$RightWeaponBone = 119
# 中段握刀的位置（Y 沿刀身，正向朝刀尖）。常态约 (0.09, 0)，Y 越大刀离手心越远
$GripX = 0.09
$GripY = 0.15
# 下界低于它的才是长枪握法（后面的块只有收招的小幅摆动，不用改）
$SpearGripBelow = -0.3
$Moves = @(
    @{ From = "a809_040000"; To = "a293_030500" },
    @{ From = "a809_040001"; To = "a293_030505" }
)
$HkxIdBase = 1000000000

$a9x = Read-Bnd (Resolve-Path $A9xSrc).Path
$a6x = Read-Bnd (Resolve-Path $A6xSrc).Path
$template = $a6x.Files | Where-Object { $_.Name.EndsWith(".hkx") } | Select-Object -First 1
# 路径是打包机上的 W:\...，这台机器没有这个盘，不能用 Join-Path
$folder = [IO.Path]::GetDirectoryName($template.Name)
foreach ($m in $Moves) {
    [byte[]]$bytes = (Find-File $a9x "\$($m.From).hkx").Bytes.ToArray()
    $patched = 0
    foreach ($block in @(Get-HkxBonePosition $bytes $RightWeaponBone)) {
        $x = $block.Channels[0]
        $y = $block.Channels[1]
        if (-not $y -or -not $y.Spline -or $y.Lo -ge $SpearGripBelow) { continue }
        Set-HkxSplineLow $bytes $x $GripX
        Set-HkxSplineLow $bytes $y $GripY
        "$($m.To): 第 $($block.Block) 块 X [$($x.Lo), $($x.Hi)] Y [$($y.Lo), $($y.Hi)] → 下界 ($GripX, $GripY)"
        $patched++
    }
    if ($patched -ne 1) { throw "$($m.From): expected one spear grip block, found $patched" }

    $id = $HkxIdBase + [int]$m.To.Substring(1, 3) * 1000000 + [int]$m.To.Substring(5)
    if ($a6x.Files | Where-Object { $_.ID -eq $id }) { throw "$($m.To) already in a6x" }
    $file = New-Object SoulsFormats.BinderFile
    $file.Flags = $template.Flags
    $file.CompressionType = $template.CompressionType
    $file.ID = $id
    $file.Name = "$folder\$($m.To).hkx"
    $file.Bytes = $bytes
    $at = 0
    while ($at -lt $a6x.Files.Count -and $a6x.Files[$at].ID -lt $id) { $at++ }
    $a6x.Files.Insert($at, $file)
}
Write-Bnd $a6x (Join-Path ([IO.Path]::GetFullPath($OutDir)) "chr\c0000_a6x.anibnd.dcx")
"c0000_a6x: 加入 $(@($Moves).Count) 段动画，共 $($a6x.Files.Count) 个文件"
