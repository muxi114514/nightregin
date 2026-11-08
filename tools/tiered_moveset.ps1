<#
  执行者8 的招式表 a293（只有 MMV，跟在 tiered_tae.ps1 之后就地改 pkg 的 chr\c0000.anibnd.dcx）。
  招式由武器字段 heroMovesetOverrideID 决定：妖刀 278，角色带锻体时 DLL 换成 293（src\crucible_sword.rs，按角色判断，队友也看得到）。
  行为图里 a293 的动作入口都在，但没有武器、事件表和动画文件在用。这里照 a278 复制一份（动画本来就都是借的），再换掉要改的招式，没锻体的人不受影响。
    重攻击     32510 按住 → 奥陶漩涡蓄力版 a798_040000，32515 松手 → 快版 a798_040001（两边都是「按住路线 + 松手路线」）。
               行为图的 a293 没有第一下的松手动作 32505，tools\tiered_buffs.lua 把第一下重攻击转到这组。
    架势重攻击 单手入口 a278 没有也用不到，新增给纳刀架势的派生（tools\tiered_buffs.lua 转过来，已解放 30500 / 30505，未解放 30510）：
               30500 按住 → 志留亚漩涡蓄力版 a809_040000，30505 松手 → 快版 a809_040001；30510 百相之羽 a857_040000（没有松手路线）
               志留亚用改过握刀位置的动画文件（tools\tiered_grip.ps1），其余直接借原动画
  都取原动作的取消、转向、特效等事件；攻击换成妖刀判定（带上执行者7 的附加攻击与遗物子弹），冲击子弹走全局判定（relics\src\tiered\tae.rs）
  五段动作整段无敌（跳转表 8）：招式表只在带锻体时切换，不需要状态条件
  重复运行会报错（已改过）
#>
param(
    [Parameter(Mandatory)] [string]$Anibnd,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

$Moveset = 293
$BaseMoveset = 278
$TaeBinderBase = 5000000
$TaeIdBase = 2000
$HkxIdScale = 1000000
# Base：a278 没有的动作照哪段复制妖刀攻击（没写就是改 a278 的同号动作）；Judge：妖刀判定（OldHit 秒那一下）
# Bullet：冲击子弹的判定（百相之羽没有子弹）；ShotDmy：子弹换挂点（原挂点在长枪枪尖，妖刀没有）；OwnHkx：用自己编号的动画文件
$Moves = @(
    @{ Anim = 32510; Tae = "a798"; Source = 40000; OldHit = 1.533; Judge = 316; Bullet = 7498; What = "按住 → 奥陶漩涡蓄力版" },
    @{ Anim = 32515; Tae = "a798"; Source = 40001; OldHit = 0.3; Judge = 310; Bullet = 7499; What = "松手 → 奥陶漩涡快版" },
    @{ Anim = 30500; Base = 32510; Tae = "a809"; Source = 40000; OldHit = 1.533; Judge = 316; Bullet = 7493; OwnHkx = $true; What = "架势重攻击（已解放）按住 → 志留亚漩涡蓄力版" },
    @{ Anim = 30505; Base = 32515; Tae = "a809"; Source = 40001; OldHit = 0.3; Judge = 310; Bullet = 7494; ShotDmy = 5120; OwnHkx = $true; What = "松手 → 志留亚漩涡快版" },
    @{ Anim = 30510; Base = 32510; Tae = "a857"; Source = 40000; OldHit = 1.533; Judge = 316; What = "架势重攻击（未解放）→ 百相之羽" }
)
# 松手判定认的效果：重攻击 100280（战技原本用 100285）
$ReleaseWindow = 100280
$SkillReleaseWindow = 100285
# 不复制：攻击与子弹另外处理；330 是战技扣 FP，妖刀的招式不扣
$DropTypes = @(1, 2, 330)
# 战技连段用的效果，妖刀重攻击不要
$SkipEffects = @(100420)
# 右手武器的挂点是 10xxx，妖刀的是 5xxx
$RightHandDmy = 10000
$SkillWeaponDmy = 5000
$DrawnState = 609
$AilmentState = 2393
$HitTolerance = 0.02
# 跳转表 8 = 无敌；结束时间同原版「到动作结束」的事件
$InvincibleJump = 8
$WholeAnimEnd = 100
# 子弹事件 +18 / +19：同原版角色技能武器的写法（来源 3），伤害按妖刀算
$BulletSourceOffset = 18
$SkillWeaponSource = [byte[]](0xFF, 3)

function Set-Int($Ev, [int]$Offset, [int]$Value) {
    $p = Get-Params $Ev
    [BitConverter]::GetBytes($Value).CopyTo($p, $Offset)
    $Ev.SetParameterBytes($false, $p, $false)
}
function To-SkillWeaponDmy($Ev, [int]$Offset) {
    $d = [BitConverter]::ToInt32((Get-Params $Ev), $Offset)
    if ($d -ge $RightHandDmy -and $d -lt $RightHandDmy + 1000) { Set-Int $Ev $Offset ($d - $RightHandDmy + $SkillWeaponDmy) }
}
# 把事件复制进动画，原来同组的事件仍然同组
function Add-Copy($Anim, $Ev, [double]$Start, [double]$End, $Groups) {
    $c = $Ev.GetClone($false)
    $c.StartTime = [single]$Start
    $c.EndTime = [single]$End
    if (-not $Groups.ContainsKey($Ev.Group)) {
        $g = $Ev.Group.GetClone()
        $Groups[$Ev.Group] = $g
        $Anim.EventGroups.Add($g)
    }
    $c.Group = $Groups[$Ev.Group]
    $Anim.Events.Add($c)
    $c
}

$path = (Resolve-Path $Anibnd).Path
$bnd = Read-Bnd $path
if ($bnd.Files | Where-Object { $_.ID -eq $TaeBinderBase + $Moveset }) { throw "a$Moveset.tae already exists (already patched?)" }
$baseFile = Find-File $bnd "\a$BaseMoveset.tae"
$tae = [SoulsFormats.TAE]::Read($baseFile.Bytes)
$tae.ID = $TaeIdBase + $Moveset
# 动画本来就借别的文件；少数自带的改成借 a278，a293 没有动画文件（整段引用别的动作的头没有这个字段，不用改）
foreach ($a in $tae.Animations) {
    if ($a.MiniHeader.GetType().Name -eq "Standard" -and -not $a.MiniHeader.ImportsHKX) {
        $a.MiniHeader.ImportsHKX = $true
        $a.MiniHeader.ImportHKXSourceAnimID = $BaseMoveset * $HkxIdScale + $a.ID
    }
}

foreach ($m in $Moves) {
    if ($m.Base) {
        $anim = Get-Anim ([SoulsFormats.TAE]::Read($baseFile.Bytes)) $m.Base
        $anim.ID = $m.Anim
        $tae.Animations.Add($anim)
    } else {
        $anim = Get-Anim $tae $m.Anim
    }
    $source = Get-Anim ([SoulsFormats.TAE]::Read((Find-File $bnd "\$($m.Tae).tae").Bytes)) $m.Source
    $sourceEvents = @($source.Events)
    $hit = $sourceEvents | Where-Object { $_.Type -eq 1 } | Select-Object -First 1
    $shot = $sourceEvents | Where-Object { $_.Type -eq 2 } | Select-Object -First 1
    if (-not $hit) { throw "$($m.Tae) $($m.Source): attack event not found" }
    if ($m.Bullet -and -not $shot) { throw "$($m.Tae) $($m.Source): bullet event not found" }

    $old = @($anim.Events)
    $anim.Events.Clear()
    $anim.EventGroups.Clear()
    $groups = New-Object 'System.Collections.Generic.Dictionary[object,object]'

    foreach ($e in $sourceEvents) {
        if ($DropTypes -contains $e.Type) { continue }
        if ($e.Type -eq 66 -and $SkipEffects -contains [BitConverter]::ToInt32((Get-Params $e), 0)) { continue }
        $c = Add-Copy $anim $e $e.StartTime $e.EndTime $groups
        if ($c.Type -in 66, 67 -and [BitConverter]::ToInt32((Get-Params $c), 0) -eq $SkillReleaseWindow) { Set-Int $c 0 $ReleaseWindow }
        if ($c.Type -in 96, 792) { To-SkillWeaponDmy $c 4 }
    }

    # 妖刀那一下与执行者7 的附加攻击对到新的出手时间，遗物子弹跟着平移
    $shift = $hit.StartTime - $m.OldHit
    $attacks = @($old | Where-Object { $_.Type -eq 1 -and [Math]::Abs($_.StartTime - $m.OldHit) -lt $HitTolerance -and
        ((Get-State $_) -eq $AilmentState -or ((Get-Judge $_) -eq $m.Judge -and (Get-State $_) -eq $DrawnState)) })
    if ($attacks.Count -lt 1 -or -not ($attacks | Where-Object { (Get-Judge $_) -eq $m.Judge })) { throw "a$BaseMoveset $($m.Anim): judge $($m.Judge) at $($m.OldHit) not found" }
    foreach ($a in $attacks) { [void](Add-Copy $anim $a $hit.StartTime $hit.EndTime $groups) }
    foreach ($b in $old | Where-Object { $_.Type -eq 2 }) { [void](Add-Copy $anim $b ($b.StartTime + $shift) ($b.EndTime + $shift) $groups) }

    if ($m.Bullet) {
        $c = Add-Copy $anim $shot $shot.StartTime $shot.EndTime $groups
        if ($m.ShotDmy) { Set-Int $c 0 $m.ShotDmy } else { To-SkillWeaponDmy $c 0 }
        Set-Int $c 8 $m.Bullet
        $p = Get-Params $c
        $SkillWeaponSource.CopyTo($p, $BulletSourceOffset)
        $c.SetParameterBytes($false, $p, $false)
    }

    $jump = $sourceEvents | Where-Object { $_.Type -eq 0 } | Select-Object -First 1
    if (-not $jump) { throw "$($m.Tae) $($m.Source): jump table event not found" }
    $c = Add-Copy $anim $jump 0 $WholeAnimEnd $groups
    Set-JumpId $c $InvincibleJump
    Set-Int $c 4 0
    Set-State $c 0

    if ($m.OwnHkx) {
        $hkx = $Moveset * $HkxIdScale + $m.Anim
        $anim.AnimFileName = "a{0}_{1:D6}.hkt" -f $Moveset, $m.Anim
        $anim.MiniHeader.ImportsHKX = $false
        $anim.MiniHeader.ImportHKXSourceAnimID = -1
    } else {
        $hkx = [int]$m.Tae.Substring(1) * $HkxIdScale + $m.Source
        $anim.AnimFileName = "{0}_{1:D6}.hkt" -f $m.Tae, $m.Source
        $anim.MiniHeader.ImportsHKX = $true
        $anim.MiniHeader.ImportHKXSourceAnimID = $hkx
    }
    "a$Moveset.tae: $($m.Anim) $($m.What)（动画 $hkx，攻击 $($attacks.Count) 个，判定 $($m.Judge) / $($m.Bullet)）"
}
# 新增的动作排回编号顺序
$sorted = @($tae.Animations | Sort-Object ID)
$tae.Animations.Clear()
foreach ($a in $sorted) { $tae.Animations.Add($a) }

$file = New-Object SoulsFormats.BinderFile
$file.Flags = $baseFile.Flags
$file.CompressionType = $baseFile.CompressionType
$file.ID = $TaeBinderBase + $Moveset
$file.Name = $baseFile.Name -replace "a$BaseMoveset\.tae$", "a$Moveset.tae"
$file.Bytes = [byte[]]$tae.Write()
$at = 0
while ($at -lt $bnd.Files.Count -and $bnd.Files[$at].ID -lt $file.ID) { $at++ }
$bnd.Files.Insert($at, $file)
Write-Bnd $bnd $path
"a$Moveset.tae: 新增招式表（$(@($tae.Animations).Count) 段动作），文件编号 $($file.ID)"
