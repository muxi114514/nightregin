<#
  执行者8 熔炉形态的变身动作：熔炉骑士 c2500 转阶段踏地（3023，剑盾版）搬到玩家骨骼（tools\hkx_retarget.py，踏地不带刀），
  放进熔炉妖刀招式表 a293 的空闲槽位 32050（双手第 6 段轻攻击；妖刀连段只有 5 段，脚本只让踏地进这一段），
  脚本（tools\tiered_crucible.lua）按绝招直接播 W_AttackBothLight6（DLL 见变身中标记切招式表），播完或到取消点拔刀。
  就地改 <Pkg>\chr：
    c0000_a6x.anibnd   加 a293_032050.hkx
    c0000.anibnd       a293.tae 加 32050 条目：0~1.2 秒无敌、禁转身，全程藏双手武器（710）；
                       借原版野兽变身起手（a907 670000）的熔炉光晕 463020（10 个挂点）与两段音效；
                       骑士原动作的踏地爆发 625071（骑士挂点 91 = 胸口 Spine2，玩家同位置是 220）与震屏，爆发同时放蓄力黄金之怒（效果 9971880）；
                       625071 在 MMV 随角色加载的 sfxbnd_c0000 里已有，不能再加进常驻包（同一特效注册两次，进圆桌加载角色就闪退）；
                       1.00 秒挂熔炉形态（效果 9971870，换套装）：换装会重建角色模型，同一帧放的特效与震屏会丢，
                       所以比骑士震屏（1.07）、爆发（1.10）早几帧，看起来是同时；被打断没挂上由脚本在踏地结束时补；
                       2.6 秒起开通用 / 移动取消（脚本见到就拔刀，原版连段不处理这段）
  效果行要先由 tiered_crucible.ps1 加好。重复运行会报错
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$Game = "$PSScriptRoot\..\..\Game",
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

$Moveset = 293
$Slot = 32050
$HkxId = 1000000000 + $Moveset * 1000000 + $Slot
$KnightAnim = "a000_003023"
$KnightTaeAnim = 3023
$KnightBurstFx = 625071
$BurstDummy = 220
$CrucibleForm = 9971870
# 与骑士爆发同时放的蓄力黄金之怒（发出效果，行由 tiered_crucible.ps1 加）
$GoldenWrath = 9971880
# 时间（秒）：骑士 1.03 秒踏地（爆发、震屏照骑士原时间）；换装在它们前几帧；之前蓄光晕，之后开放取消
$Stomp = 1.03
$FormAt = 1.00
$GlowStart = 0.30
$Guarded = 1.2
$CancelAt = 2.6
$UntilEnd = 100.0
# 跳转表：8 无敌、7 禁转身；4 通用取消、11 移动取消（脚本 IsGeneralAnimCancelPossible / IsMoveCancelPossible 认）
$JumpInvincible = 8
$JumpNoTurn = 7
$CancelJumps = 4, 11
$HideBothHands = [byte[]](1, 1, 0, 0)
# 借原版野兽变身起手的熔炉光晕与音效
$BeastStart = 670000
$GlowFx = 463020
$StartSound = 807501
$BurstSound = 808000

$pkgPath = (Resolve-Path $Pkg).Path
$vanilla = Join-Path $PSScriptRoot "..\target\vanilla"
$need = @("/chr/c2500.anibnd.dcx", "/chr/c2500_div00.anibnd.dcx") | Where-Object { -not (Test-Path (Join-Path $vanilla ($_.TrimStart("/") -replace "/", "\"))) }
if ($need) { & (Join-Path $PSScriptRoot "extract_vanilla.ps1") -Game $Game -OutDir $vanilla -Paths $need | Out-Null }

$aniPath = "$pkgPath\chr\c0000.anibnd.dcx"
$a6xPath = "$pkgPath\chr\c0000_a6x.anibnd.dcx"
$ani = Read-Bnd $aniPath
$knight = Read-Bnd (Join-Path $vanilla "chr\c2500.anibnd.dcx")
$knightDiv = Read-Bnd (Join-Path $vanilla "chr\c2500_div00.anibnd.dcx")

# ---- 动画：骑士踏地 → 玩家骨骼 ----
$tmp = [IO.Path]::GetTempFileName()
$files = @{ Anim = "$tmp.anim.hkx"; Src = "$tmp.src.hkx"; Dst = "$tmp.dst.hkx"; Out = "$tmp.out.hkx" }
try {
    [IO.File]::WriteAllBytes($files.Anim, (Find-File $knightDiv "\$KnightAnim.hkx").Bytes.ToArray())
    [IO.File]::WriteAllBytes($files.Src, (Find-File $knight "\skeleton.hkx").Bytes.ToArray())
    [IO.File]::WriteAllBytes($files.Dst, (Find-File $ani "\skeleton.hkx").Bytes.ToArray())
    & python -I (Join-Path $PSScriptRoot "hkx_retarget.py") $files.Anim $files.Src $files.Dst $files.Out | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "hkx_retarget.py failed" }
    [byte[]]$stompHkx = [IO.File]::ReadAllBytes($files.Out)
} finally { Remove-Item $tmp, $files.Values -ErrorAction SilentlyContinue }

$a6x = Read-Bnd $a6xPath
if ($a6x.Files | Where-Object { $_.ID -eq $HkxId }) { throw "a${Moveset}_0$Slot already in a6x" }
$template = $a6x.Files | Where-Object { $_.Name.EndsWith(".hkx") } | Select-Object -First 1
$file = New-Object SoulsFormats.BinderFile
$file.Flags = $template.Flags
$file.CompressionType = $template.CompressionType
$file.ID = $HkxId
$file.Name = [IO.Path]::GetDirectoryName($template.Name) + ("\a{0}_{1:D6}.hkx" -f $Moveset, $Slot)
$file.Bytes = $stompHkx
$at = 0
while ($at -lt $a6x.Files.Count -and $a6x.Files[$at].ID -lt $HkxId) { $at++ }
$a6x.Files.Insert($at, $file)

# ---- TAE 条目 ----
$taeFile = Find-File $ani "\a$Moveset.tae"
$tae = [SoulsFormats.TAE]::Read($taeFile.Bytes)
if ($tae.Animations | Where-Object { $_.ID -eq $Slot }) { throw "a$Moveset.tae already has $Slot" }
$heavyEnd = Get-Anim $tae 32515
$beast = Get-Anim ([SoulsFormats.TAE]::Read((Find-File $ani "\a907.tae").Bytes)) $BeastStart
$knightTae = Get-Anim ([SoulsFormats.TAE]::Read((Find-File $knight "\c2500.tae").Bytes)) $KnightTaeAnim
$gesture = Get-Anim ([SoulsFormats.TAE]::Read((Find-File $ani "\a00.tae").Bytes)) 5850

$anim = Get-Anim ([SoulsFormats.TAE]::Read($taeFile.Bytes)) 32515
$anim.ID = $Slot
$anim.AnimFileName = "a{0}_{1:D6}.hkt" -f $Moveset, $Slot
$anim.MiniHeader.ImportsHKX = $false
$anim.MiniHeader.ImportHKXSourceAnimID = -1
$anim.Events.Clear()
$anim.EventGroups.Clear()
$groups = New-Object 'System.Collections.Generic.Dictionary[object,object]'
function Add-Event($Source, [double]$Start, [double]$End) {
    $c = $Source.GetClone($false)
    $c.StartTime = [single]$Start
    $c.EndTime = [single]$End
    if (-not $groups.ContainsKey($Source.Group)) {
        $g = $Source.Group.GetClone()
        $groups[$Source.Group] = $g
        $anim.EventGroups.Add($g)
    }
    $c.Group = $groups[$Source.Group]
    $anim.Events.Add($c)
    $c
}
function First-Event($From, [int]$Type, [scriptblock]$Where = { $true }) {
    $e = $From.Events | Where-Object { $_.Type -eq $Type -and (& $Where $_) } | Select-Object -First 1
    if (-not $e) { throw "template event $Type not found in $($From.ID)" }
    $e
}
function Set-Bytes($Ev, [int]$Offset, [byte[]]$Value) {
    $p = Get-Params $Ev
    $Value.CopyTo($p, $Offset)
    $Ev.SetParameterBytes($false, $p, $false)
}
function Add-Jump([int]$Id, [double]$Start, [double]$End) {
    $c = Add-Event (First-Event $heavyEnd 0) $Start $End
    Set-JumpId $c $Id
    Set-State $c 0
}

Add-Jump $JumpInvincible 0 $Guarded
Add-Jump $JumpNoTurn 0 $Guarded
foreach ($j in $CancelJumps) { Add-Jump $j $CancelAt $UntilEnd }
Set-Bytes (Add-Event (First-Event $gesture 710) 0 $UntilEnd) 0 $HideBothHands
# 光晕在踏地前聚起来，踏地时爆发
$glow = @($beast.Events | Where-Object { $_.Type -eq 96 -and [BitConverter]::ToInt32((Get-Params $_), 0) -eq $GlowFx })
if ($glow.Count -lt 1) { throw "glow FFX $GlowFx not found in a907 $BeastStart" }
foreach ($e in $glow) { [void](Add-Event $e $GlowStart $FormAt) }
$form = Add-Event (First-Event $beast 66) $FormAt ($FormAt + 0.1)
Set-Bytes $form 0 ([BitConverter]::GetBytes([int]$CrucibleForm))
# 骑士的爆发与震屏照原时间（爆发 1.10、震屏 1.07），爆发改挂玩家胸口
$burst = First-Event $knightTae 96 { [BitConverter]::ToInt32((Get-Params $args[0]), 0) -eq $KnightBurstFx }
Set-Bytes (Add-Event $burst $burst.StartTime $burst.EndTime) 4 ([BitConverter]::GetBytes([int]$BurstDummy))
# 黄金之怒只在本机挂发出效果（67），子弹由游戏同步给队友
$wrath = Add-Event (First-Event $beast 67) $burst.StartTime ($burst.StartTime + 0.1)
Set-Bytes $wrath 0 ([BitConverter]::GetBytes([int]$GoldenWrath))
$rumble = First-Event $knightTae 144
[void](Add-Event $rumble $rumble.StartTime $rumble.EndTime)
foreach ($s in @(@{ Id = $StartSound; At = $GlowStart }, @{ Id = $BurstSound; At = $Stomp })) {
    $sound = $beast.Events | Where-Object { $_.Type -eq 129 -and [BitConverter]::ToInt32((Get-Params $_), 4) -eq $s.Id } | Select-Object -First 1
    if (-not $sound) { throw "sound $($s.Id) not found in a907 $BeastStart" }
    [void](Add-Event $sound $s.At ($s.At + ($sound.EndTime - $sound.StartTime)))
}

$tae.Animations.Add($anim)
$sorted = @($tae.Animations | Sort-Object ID)
$tae.Animations.Clear()
foreach ($a in $sorted) { $tae.Animations.Add($a) }
$taeFile.Bytes = [byte[]]$tae.Write()
Write-Bnd $ani $aniPath
Write-Bnd $a6x $a6xPath
"a$Moveset.tae: 加 $Slot 熔炉形态变身（骑士踏地，事件 $($anim.Events.Count) 个），a6x 加 a${Moveset}_0$Slot（$($stompHkx.Length) 字节）"
