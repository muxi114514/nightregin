<#
  三档锻体的动作文件部分（只有 MMV，跟在 relic_assets.ps1 之后就地改 pkg 的 chr\c0000.anibnd.dcx）。
  判定号 → 行为号、状态值与 relics\src\tiered\tae.rs 对应，改了要同步：
    复制      照原版遗物的条件事件复制一份（同类型、同时间、各占一个事件组），换成新判定号与锻体标记的状态值
              铁之眼3 技艺追加判定 6120（状态 2393）→ 6125；铁之眼4 绝招毒箭 7140（2258）→ 腐败箭 7146，
              另把普通气流 7105（2025）复制成认腐败标记的版本；学者5 绝招连结 7401（2028）→ 7402；
              守护者7 冰风暴：技艺 a901 510000 / 510005 的普通旋风 6050 / 6055（2004）→ 6930 / 6931（2393），
              遗物延长旋风 6084 / 6086（2037）→ 6932 / 6933（2258），两个状态由 DLL 二选一挂（relics\src\tiered\guardian.rs）
    还量表    扣量表事件（类型 1000）之后 0.1 秒发一颗只给自己挂「待返还」标记的子弹，以同 TAE 里判定 7490 的事件为模板（身体挂点、不占子弹槽位）；
              扣量表同时开的量表锁定（类型 1002）期间加量表无效，由 DLL 等「绝招中」消失再还（relics\src\tiered\tae.rs）
              追踪者3 非蓄力绝招 600005 / 605000 → 判定 7496；送葬者6 强化技艺 599010 → 7497
    叠加      某个 TAE 里用技艺武器出招的攻击事件（类型 1、判定号 < 1000、参数第 13 字节为 3）各复制几份，换成新判定号与状态值，
              攻击编号（+4）换成本动画没用过的（同编号的攻击共用命中名单，前几版就因此打不中）
              执行者7 执行者刀类招式 a277 / a278 的妖刀攻击（原版多为认状态 609 的拔刀版）→ 全局判定 6998 / 6999
             （同原版铁之眼遗物的 6120，不随武器变体；状态借用该遗物的 2393，锻体标记直接带）
    跳转表    给一段动画整段加一条认状态的跳转表事件（参数 +0 编号、+14 状态），照原版女爵遗物 a903 539000 的无敌（跳转表 8 + 状态 2394，0~100）
              执行者8 解放斩 a907 571020 → 跳转表 8、状态 2185（未解放的斩击是 571025，不加）
  每处都要求找到原版事件，MMV 更新后对不上会直接报错；重复运行会报错（已改过）
#>
param(
    [Parameter(Mandatory)] [string]$Anibnd,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

$Frame = 1.0 / 30
$Copies = @(
    @{ Tae = "a902"; Anims = @(520000, 520005, 525000, 525005); Type = 1; Judge = 6120; State = 2393; NewJudge = 6125; NewState = 2180; What = "铁之眼3 技艺腐败" },
    @{ Tae = "a902"; Anims = @(620000); Type = 2; Judge = 7140; State = 2258; NewJudge = 7146; NewState = 2181; What = "铁之眼4 腐败箭" },
    @{ Tae = "a902"; Anims = @(620000); Type = 2; Judge = 7105; State = 2025; NewJudge = 7105; NewState = 2181; What = "铁之眼4 气流" },
    @{ Tae = "a908"; Anims = @(680000); Type = 2; Judge = 7401; State = 2028; NewJudge = 7402; NewState = 2182; What = "学者5 绝招连结" },
    @{ Tae = "a901"; Anims = @(510000); Type = 2; Judge = 6050; State = 2004; NewJudge = 6930; NewState = 2393; What = "守护者7 冰风暴（蓄力）" },
    @{ Tae = "a901"; Anims = @(510005); Type = 2; Judge = 6055; State = 2004; NewJudge = 6931; NewState = 2393; What = "守护者7 冰风暴" },
    @{ Tae = "a901"; Anims = @(510000); Type = 2; Judge = 6084; State = 2037; NewJudge = 6932; NewState = 2258; What = "守护者7 冰风暴（延长、蓄力）" },
    @{ Tae = "a901"; Anims = @(510005); Type = 2; Judge = 6086; State = 2037; NewJudge = 6933; NewState = 2258; What = "守护者7 冰风暴（延长）" }
)
$Refunds = @(
    @{ Tae = "a900"; Anims = @(600005, 605000); TemplateAnim = 600005; NewJudge = 7496; NewState = 2183; What = "追踪者3 非蓄力绝招还量表" },
    @{ Tae = "a909"; Anims = @(599010); TemplateAnim = 690000; NewJudge = 7497; NewState = 2184; What = "送葬者6 强化技艺还量表" }
)
$Overlays = @(
    @{ Tae = "a277"; NewJudges = @(6998, 6999); NewState = 2393; Min = 12; What = "执行者7 妖刀附加全异常" },
    @{ Tae = "a278"; NewJudges = @(6998, 6999); NewState = 2393; Min = 25; What = "执行者7 妖刀附加全异常" }
)
# 攻击事件参数第 13 字节：3 = 用技艺武器出招（执行者妖刀的拔刀版事件、妖刀跳攻等），0 = 手上的武器
$SkillWeaponOffset = 13
$SkillWeaponFlag = 3
# 原版攻击编号最大用到 7
$MaxAttackIndex = 7
$RefundTemplateJudge = 7490
$GaugeEventType = 1000
$WeaponJudgeLimit = 1000

function Find-Event($Anim, [int]$Type, [int]$Judge, [int]$State) {
    $Anim.Events | Where-Object { $_.Type -eq $Type -and (Get-Judge $_) -eq $Judge -and (Get-State $_) -eq $State } | Select-Object -First 1
}
function Assert-New($Anim, [int]$Type, [int]$Judge, [int]$State) {
    if (Find-Event $Anim $Type $Judge $State) { throw "anim $($Anim.ID): event judge $Judge state $State already exists (already patched?)" }
}

# Template：同动画里当模板的跳转表（只取事件结构）；事件时间 0~100 表示到动画结束或被取消为止
$Jumps = @(
    @{ Tae = "a907"; Anim = 571020; Template = 24; Jump = 8; State = 2185; What = "执行者8 解放斩无敌" }
)
$JumpWholeAnim = 100.0

$path = (Resolve-Path $Anibnd).Path
$bnd = Read-Bnd $path
$taes = @{}
foreach ($name in (@($Copies) + @($Refunds) + @($Overlays) + @($Jumps) | ForEach-Object { $_.Tae } | Sort-Object -Unique)) {
    $file = Find-File $bnd "\$name.tae"
    $bytes = $file.Bytes.ToArray()
    $tae = [SoulsFormats.TAE]::Read($bytes)
    # 读写往返不一致的 TAE 改了可能丢东西，先报出来
    if (-not [Linq.Enumerable]::SequenceEqual([byte[]]$tae.Write(), [byte[]]$bytes)) { "  警告：$name.tae 读写往返不一致" }
    $taes[$name] = @{ File = $file; Tae = $tae }
}

foreach ($c in $Copies) {
    $tae = $taes[$c.Tae].Tae
    foreach ($id in $c.Anims) {
        $anim = Get-Anim $tae $id
        $old = Find-Event $anim $c.Type $c.Judge $c.State
        if (-not $old) { throw "$($c.Tae) anim ${id}: event type $($c.Type) judge $($c.Judge) state $($c.State) not found" }
        Assert-New $anim $c.Type $c.NewJudge $c.NewState
        $anim.Events.Add((New-BulletEvent $old $c.NewJudge $c.NewState $old.StartTime (New-Group $anim $old)))
    }
    "$($c.Tae).tae: $($c.What)（判定 $($c.Judge) → $($c.NewJudge)，状态 $($c.NewState)）x$($c.Anims.Count)"
}

foreach ($r in $Refunds) {
    $tae = $taes[$r.Tae].Tae
    $template = (Get-Anim $tae $r.TemplateAnim).Events | Where-Object { $_.Type -eq 2 -and (Get-Judge $_) -eq $RefundTemplateJudge -and (Get-State $_) -eq 0 } |
        Select-Object -First 1
    if (-not $template) { throw "$($r.Tae) anim $($r.TemplateAnim): judge $RefundTemplateJudge event not found" }
    foreach ($id in $r.Anims) {
        $anim = Get-Anim $tae $id
        $gauge = $anim.Events | Where-Object { $_.Type -eq $GaugeEventType } | Select-Object -First 1
        if (-not $gauge) { throw "$($r.Tae) anim ${id}: gauge event $GaugeEventType not found" }
        Assert-New $anim 2 $r.NewJudge $r.NewState
        $anim.Events.Add((New-BulletEvent $template $r.NewJudge $r.NewState ($gauge.StartTime + 3 * $Frame) (New-Group $anim $template)))
    }
    "$($r.Tae).tae: $($r.What)（判定 $($r.NewJudge)，状态 $($r.NewState)）x$($r.Anims.Count)"
}

foreach ($o in $Overlays) {
    $n = 0
    foreach ($anim in $taes[$o.Tae].Tae.Animations) {
        $attacks = @($anim.Events | Where-Object { $_.Type -eq 1 -and (Get-Params $_)[$SkillWeaponOffset] -eq $SkillWeaponFlag -and (Get-Judge $_) -ge 0 -and
            (Get-Judge $_) -lt $WeaponJudgeLimit })
        if ($anim.Events | Where-Object { $_.Type -eq 1 -and (Get-State $_) -eq $o.NewState }) { throw "$($o.Tae) anim $($anim.ID): state $($o.NewState) already exists (already patched?)" }
        # 每个新判定占一个本动画没用过的攻击编号，否则和原版那一下共用命中名单，打中过的敌人不会再被打中
        $used = @($anim.Events | Where-Object { $_.Type -eq 1 } | ForEach-Object { Get-AttackIndex $_ })
        $free = @(1..$MaxAttackIndex | Where-Object { $used -notcontains $_ } | Select-Object -First $o.NewJudges.Count)
        if ($attacks.Count -gt 0 -and $free.Count -lt $o.NewJudges.Count) { throw "$($o.Tae) anim $($anim.ID): no free attack index" }
        foreach ($ev in $attacks) {
            for ($k = 0; $k -lt $o.NewJudges.Count; $k++) {
                $copy = New-BulletEvent $ev $o.NewJudges[$k] $o.NewState $ev.StartTime (New-Group $anim $ev)
                Set-AttackIndex $copy $free[$k]
                $anim.Events.Add($copy)
            }
            $n++
        }
    }
    # 数量骤减说明 MMV 改了执行者的刀类动作，要重新核对
    if ($n -lt $o.Min) { throw "$($o.Tae).tae: only $n skill weapon attack events (expected >= $($o.Min))" }
    "$($o.Tae).tae: $($o.What)（判定 $($o.NewJudges -join ' / ')，状态 $($o.NewState)）x$n"
}

foreach ($j in $Jumps) {
    $anim = Get-Anim $taes[$j.Tae].Tae $j.Anim
    if ($anim.Events | Where-Object { $_.Type -eq 0 -and (Get-State $_) -eq $j.State }) { throw "$($j.Tae) anim $($j.Anim): state $($j.State) already exists (already patched?)" }
    $template = $anim.Events | Where-Object { $_.Type -eq 0 -and (Get-JumpId $_) -eq $j.Template } | Select-Object -First 1
    if (-not $template) { throw "$($j.Tae) anim $($j.Anim): jump table $($j.Template) not found" }
    $ev = $template.GetClone($false)
    $ev.StartTime = 0
    $ev.EndTime = $JumpWholeAnim
    $ev.Group = New-Group $anim $template
    Set-JumpId $ev $j.Jump
    Set-State $ev $j.State
    $anim.Events.Add($ev)
    "$($j.Tae).tae: $($j.What)（跳转表 $($j.Jump)，状态 $($j.State)）"
}

foreach ($t in $taes.Values) { $t.File.Bytes = [byte[]]$t.Tae.Write() }
Write-Bnd $bnd $path
