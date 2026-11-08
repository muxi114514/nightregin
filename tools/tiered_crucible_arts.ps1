<#
  执行者8 熔炉形态的变招（跟在 tiered_crucible_anim.ps1 之后，就地改 <Pkg>\chr）：
    c0000.anibnd   a293.tae 加三个条目，动画借原版祷告的施法动作（之尾 a470、之角 a471；动作段 = 400 + 魔法 refType）：
                     32950 双手特殊 1        ← 蓄力之尾 a470_045010（扫两次）：蓄力奥陶漩涡收招再按重击
                     32501 双手重攻击副起手  ← 普通之尾 a470_045011：快版奥陶漩涡收招再按重击（熔炉妖刀拔刀时原版走不到这个状态）
                     32955 双手特殊 2        ← 蓄力之角 a471_045010：未解放时的架势轻攻击
                   施法事件 T64 按「当前法术」出伤害，执行者手上没有这两个法术，所以按它的引用序号换成妖刀攻击事件 T1（伤害来源 = 技能武器），
                   判定号走全局 699x（行与倍率在 tiered_crucible.ps1）；施法时减伤的武器词条与法术蓄力窗口去掉，其余（特效、音效、取消窗口等）照搬；
                   之尾另叠加熔炉骑士扫尾（c2500 3011 普通 / 3012 两扫）沿尾巴的光效 6250xx：骑士挂点 820 / 821 / 825 / 826 在尾巴骨骼上，
                   玩家同号挂点在施法动画甩的 Xtra 骨骼上，照搬挂点；时间按离它最近的那一扫对齐到之尾的出招（特效在 MMV 随角色加载的 sfxbnd_c0000 里）
    c0000.behbnd   双手特殊 1 / 2 的选择器没有 a293 片段，用 hkb_add_clips.py 补上（双手重攻击副起手原本就有）
  触发与状态接管在 tools\tiered_crucible.lua。重复运行会报错
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

$Moveset = 293
$HkxIdScale = 1000000
# Hits：施法事件 T64 参数 +8 的引用序号（0~3 = 法术 refId1~4）→ 全局判定；不在表里的（9、助手子弹）不要
$Arts = @(
    @{ Anim = 32950; Tae = "a470"; Source = 45010; Hits = @{ 2 = 6990; 1 = 6991 }; Knight = 3012; What = "蓄力之尾" },
    @{ Anim = 32501; Tae = "a470"; Source = 45011; Hits = @{ 0 = 6990 }; Knight = 3011; What = "普通之尾" },
    @{ Anim = 32955; Tae = "a471"; Source = 45010; Hits = @{ 2 = 6992 }; What = "蓄力之角" }
)
# 熔炉骑士沿尾巴的光效（c2500 扫尾动作里的特效事件 96）
$KnightTailFx = @(625050, 625051, 625052, 625060, 625061, 625062)
$FxEvent = 96
$NewClips = @("a293_032950", "a293_032955")
$CastEvent = 64
$CastRefOffset = 8
# 去掉的效果：施法时减伤（武器词条）、法术蓄力窗口
$DropEffects = @(8330901, 8330911, 8330921, 100610)
# 攻击事件模板：熔炉妖刀轻攻击里伤害来源为技能武器（+12/+13 = FF 03）的那条；+4 攻击编号、+8 判定、+14 状态条件
$TemplateAnim = 32000
$SkillWeaponSource = 3
$SourceOffset = 13

$pkgPath = (Resolve-Path $Pkg).Path
$aniPath = "$pkgPath\chr\c0000.anibnd.dcx"
$behPath = "$pkgPath\chr\c0000.behbnd.dcx"
$ani = Read-Bnd $aniPath
$taeFile = Find-File $ani "\a$Moveset.tae"
$tae = [SoulsFormats.TAE]::Read($taeFile.Bytes)
foreach ($a in $Arts) { if ($tae.Animations | Where-Object { $_.ID -eq $a.Anim }) { throw "a$Moveset.tae already has $($a.Anim)" } }
$hitTemplate = (Get-Anim $tae $TemplateAnim).Events | Where-Object { $_.Type -eq 1 -and (Get-Params $_)[$SourceOffset] -eq $SkillWeaponSource } | Select-Object -First 1
if (-not $hitTemplate) { throw "a$Moveset ${TemplateAnim}: skill weapon attack event not found" }
$shellTemplate = Get-Anim $tae 32515

function Copy-Event($Anim, $Ev, $Groups, [double]$Start, [double]$End) {
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

# 骑士扫尾的尾巴光效搬进之尾：每个特效按离它最近的那一扫平移到之尾对应那一扫的出招时间
$knightTae = [SoulsFormats.TAE]::Read((Find-File (Read-Bnd (Resolve-Path "$PSScriptRoot\..\target\vanilla\chr\c2500.anibnd.dcx").Path) "\c2500.tae").Bytes)
function Add-KnightTailFx($Anim, $Groups, [int]$KnightAnim) {
    $knight = Get-Anim $knightTae $KnightAnim
    $knightHits = @($knight.Events | Where-Object { $_.Type -eq 1 } | Sort-Object StartTime | ForEach-Object { $_.StartTime })
    $ourHits = @($Anim.Events | Where-Object { $_.Type -eq 1 } | Sort-Object StartTime | ForEach-Object { $_.StartTime })
    if ($knightHits.Count -ne $ourHits.Count) { throw "c2500 ${KnightAnim}: $($knightHits.Count) hits vs $($ourHits.Count) in a$Moveset $($Anim.ID)" }
    $n = 0
    foreach ($e in $knight.Events | Where-Object { $_.Type -eq $FxEvent -and $KnightTailFx -contains [BitConverter]::ToInt32((Get-Params $_), 0) }) {
        $nearest = 0
        for ($k = 1; $k -lt $knightHits.Count; $k++) { if ([Math]::Abs($knightHits[$k] - $e.StartTime) -lt [Math]::Abs($knightHits[$nearest] - $e.StartTime)) { $nearest = $k } }
        $shift = $ourHits[$nearest] - $knightHits[$nearest]
        # 0.0 不能写成 0：整数重载会把时间四舍五入
        [void](Copy-Event $Anim $e $Groups ([Math]::Max(0.0, [double]$e.StartTime + $shift)) ([double]$e.EndTime + $shift))
        $n++
    }
    if (-not $n) { throw "c2500 ${KnightAnim}: no tail fx found" }
    $n
}

$sources = @{}
foreach ($art in $Arts) {
    if (-not $sources.ContainsKey($art.Tae)) { $sources[$art.Tae] = [SoulsFormats.TAE]::Read((Find-File $ani "\$($art.Tae).tae").Bytes) }
    $source = Get-Anim $sources[$art.Tae] $art.Source
    $anim = Get-Anim ([SoulsFormats.TAE]::Read($taeFile.Bytes)) $shellTemplate.ID
    $anim.ID = $art.Anim
    $anim.AnimFileName = "{0}_{1:D6}.hkt" -f $art.Tae, $art.Source
    $anim.MiniHeader.ImportsHKX = $true
    $anim.MiniHeader.ImportHKXSourceAnimID = [int]$art.Tae.Substring(1) * $HkxIdScale + $art.Source
    $anim.Events.Clear()
    $anim.EventGroups.Clear()
    $groups = New-Object 'System.Collections.Generic.Dictionary[object,object]'
    $index = 0; $hits = 0
    foreach ($e in ($source.Events | Sort-Object StartTime)) {
        if ($e.Type -in 66, 67 -and $DropEffects -contains [BitConverter]::ToInt32((Get-Params $e), 0)) { continue }
        if ($e.Type -ne $CastEvent) { [void](Copy-Event $anim $e $groups $e.StartTime $e.EndTime); continue }
        $ref = (Get-Params $e)[$CastRefOffset]
        if (-not $art.Hits.ContainsKey([int]$ref)) { continue }
        $hit = Copy-Event $anim $hitTemplate $groups $e.StartTime $e.EndTime
        Set-AttackIndex $hit $index
        $p = Get-Params $hit
        [BitConverter]::GetBytes([int]$art.Hits[[int]$ref]).CopyTo($p, 8)
        $hit.SetParameterBytes($false, $p, $false)
        Set-State $hit 0
        $index++; $hits++
    }
    if ($hits -ne $art.Hits.Count) { throw "$($art.Tae) $($art.Source): expected $($art.Hits.Count) cast events, got $hits" }
    $glows = 0
    if ($art.Knight) { $glows = Add-KnightTailFx $anim $groups $art.Knight }
    $tae.Animations.Add($anim)
    "a$Moveset $($art.Anim) ← $($art.Tae)_0$($art.Source) $($art.What)：事件 $($anim.Events.Count) 个，攻击 $hits 次，骑士尾巴光效 $glows 个"
}
$sorted = @($tae.Animations | Sort-Object ID)
$tae.Animations.Clear()
foreach ($a in $sorted) { $tae.Animations.Add($a) }
$taeFile.Bytes = [byte[]]$tae.Write()

# 行为图：双手特殊 1 / 2 补 a293 片段
$graph = "\behaviors\c0000.hkx"
$beh = Read-Bnd $behPath
$file = Find-File $beh $graph
$latin = [Text.Encoding]::GetEncoding(28591)
$text = $latin.GetString($file.Bytes.ToArray())
foreach ($c in $NewClips) { if ($text.Contains($c)) { throw "behavior already has $c" } }
$tmp = [IO.Path]::GetTempFileName()
$in = "$tmp.in.hkx"; $out = "$tmp.out.hkx"; $list = "$tmp.txt"
try {
    [IO.File]::WriteAllBytes($in, $file.Bytes.ToArray())
    [IO.File]::WriteAllLines($list, [string[]]$NewClips)
    & python -I (Join-Path $PSScriptRoot "hkb_add_clips.py") $in $out $list | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "hkb_add_clips failed" }
    $file.Bytes = [IO.File]::ReadAllBytes($out)
} finally { Remove-Item $tmp, $in, $out, $list -ErrorAction SilentlyContinue }
$after = $latin.GetString($file.Bytes.ToArray())
foreach ($c in $NewClips) { if (-not $after.Contains($c)) { throw "self-check: $c not added" } }

Write-Bnd $ani $aniPath
Write-Bnd $beh $behPath
"c0000.anibnd " + (Get-FileHash $aniPath).Hash.Substring(0, 8) + "，c0000.behbnd " + (Get-FileHash $behPath).Hash.Substring(0, 8)
