<#
  自定义遗物词条的非参数部分（参数行见 relics\src\content\），从指定的原文件生成到 pkg（MMV 整合包与原版独立版共用）：
    chr\c0000.anibnd.dcx       a900.tae（追踪者）：追加攻击动画的火焰附魔与刀身火焰特效改为带状态条件的子弹事件，并加灵火 / 血焰 / 黑焰 / 命定之死版；
                               追加攻击出刀段加霸体（不依赖词条）；
                               绝招动画复制原版遗物地火事件为各元素版，大爆炸与余火按替换标记分火焰 / 灵火
    action\script\c0000.hks    追加攻击的 5 处入口判断改查入口效果（源码文本或字节码都可）
    msg\<语言>\*.msgbnd.dcx     词条名、常驻效果名与说明、状态界面效果名、附魔 buff 名、大山羊护符说明、改了效果的锻体与三档锻体的名字与说明
  来源：MMV 版取整合包的 mod\（动作、HKS）与 modKongCheng\msg（文本），独立版取 tools\extract_vanilla.ps1 导出的原版文件。
  依赖 ERBM 自带的 Andre.SoulsFormats（读写 TAE / BND / FMG / DCX，KRAK 压缩要 oo2core）
#>
param(
    [Parameter(Mandatory)] [string]$AnibndSrc,
    [Parameter(Mandatory)] [string]$HksSrc,
    # 其下为 <语言>\<名称>.msgbnd.dcx
    [Parameter(Mandatory)] [string]$MsgSrcDir,
    [Parameter(Mandatory)] [string]$OutDir,
    # 同时写角色自带能力的说明（参数只有 mmv_relics --tiered-buffs 才改，独立版不写）
    [switch]$TieredAbilities,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path

# ---- 追踪者灵火 / 血焰 / 黑焰 / 命定之死词条（ID 与 relics\src\content\wylder_ghostflame.rs、wylder_elements.rs 对应）----
$FollowUpAnims = @(500020, 500021)
# 追加攻击动画里无条件的事件（按 类型 + 参数首个整数 定位）→ 带状态条件的子弹事件：原版标记 2027 走火焰（Fire：原事件的值 → 判定，
# 沿用原动画施加的效果），灵火 2190、血焰 2194、黑焰 2195、命定之死 2196 走各自版本。67 = 施加附魔（右 / 左手），96 = 刀身火焰特效；
# 空城 0.46 把第二段（500021）的附魔改成了血焰刀刃 7020005 / 7020009，火焰分支对应判定 6046 / 6047（relics\src\content\kongcheng_follow_up.rs）。
# Delay 为推后的帧数：各元素依次晚 1~4 帧，同时带多种词条时附魔以后施加的为准
$FollowUpReplace = @(
    @{ Type = 67; Fire = @{ 7020001 = 6040; 7020005 = 6046 }; Events = @(@{ Judge = 6042; State = 2190; Delay = 1 },
        @{ Judge = 6170; State = 2194; Delay = 2 }, @{ Judge = 6173; State = 2195; Delay = 3 }, @{ Judge = 6176; State = 2196; Delay = 4 }) },
    @{ Type = 67; Fire = @{ 7020003 = 6041; 7020009 = 6047 }; Events = @(@{ Judge = 6043; State = 2190; Delay = 1 },
        @{ Judge = 6171; State = 2194; Delay = 2 }, @{ Judge = 6174; State = 2195; Delay = 3 }, @{ Judge = 6177; State = 2196; Delay = 4 }) },
    @{ Type = 96; Fire = @{ 440673 = 6044 }; Events = @(@{ Judge = 6045; State = 2190; Delay = 0 },
        @{ Judge = 6172; State = 2194; Delay = 2 }, @{ Judge = 6175; State = 2195; Delay = 3 }, @{ Judge = 6178; State = 2196; Delay = 4 }) }
)
$Frame = 1.0 / 30
# 追加攻击出刀段霸体：照抄绝招 600000 整段挂的跳转表 24（霸体）事件，窗口对齐原版韧性事件 795（开始到出刀结束）
$FollowUpArmor = @{ TemplateAnim = 600000; JumpTable = 24; WindowEvent = 795 }
# 绝招里原版遗物地火事件（判定 7030、条件 2016）复制为各元素版（灵火 / 血焰 / 黑焰 / 命定之死），与原事件同组同帧
$Ultimate = @{ TemplateAnim = 600000; Judge = 7030; State = 2016
    Copies = @(@{ Judge = 7029; State = 2191 }, @{ Judge = 7054; State = 2197 }, @{ Judge = 7055; State = 2198 }, @{ Judge = 7056; State = 2199 }) }
# 绝招大爆炸：照搬铁之眼毒箭遗物的替换机制（同原版 a902 绝招 620000 的写法：新增事件各占一个事件组，标记事件只有 1 帧）。
# 动画开头施加火焰爆炸标记 7010560（状态 2192），灵火词条每 0.1 秒施加同类别的灵火爆炸标记（状态 2193）顶掉它；
# 原爆炸事件改要求 2192 并加要求 2193 的灵火版；爆炸后的余火特效 460002 换成要求 2192 的子弹事件（按余火时长选判定），
# 以同动画里原版判定 7490 的事件为模板（身体挂点 220、不占子弹槽位），并晚爆炸 1 帧，不和爆炸同帧同挂点发射
$Blast = @{
    Anims = @(600000, 600005, 600100, 600200, 605000)
    MarkFrom = 6999615
    RegularMark = 7010560
    RegularState = 2192
    GhostState = 2193
    Judges = @{ 7001 = 7002; 7006 = 7007; 7015 = 7017 }
    AfterfireFx = 460002
    AfterfireTemplateJudge = 7490
    AfterfireJudges = @{ "1.200" = 7003; "0.800" = 7004; "1.100" = 7005; "0.833" = 7008 }
}
# 追加攻击入口：原版只认火焰追斩标记 7020000，改查入口效果（与 relics\src\content\follow_up_gate.rs 的 GATE 对应，各词条标记周期施加它）
$HksGate = @{ From = 7020000; To = 7020090; Count = 5 }
$Texts = @(
    @{ Bnd = "item_dlc01"; Fmg = 44; Id = 7020050
       zhocn = "【追踪者】发动技艺时，轻攻击能使出`n　缠绕灵火的追加攻击（仅限大剑）"
       engus = "[Wylder] Standard attacks enhanced with ghostflame`nfollow-ups when using Character Skill (greatsword only)" },
    @{ Bnd = "item_dlc01"; Fmg = 44; Id = 7010550
       zhocn = "【追踪者】发动绝招时，能燃起灵火、蔓延周围"
       engus = "[Wylder] Art activation spreads ghostflame in area" },
    @{ Bnd = "item_dlc01"; Fmg = 419; Id = 9100097; zhocn = "【追踪者】发动绝招时，能燃起灵火、蔓延周围"; engus = "Art activation spreads ghostflame in area" },
    @{ Bnd = "item_dlc01"; Fmg = 420; Id = 9100097; zhocn = "发动绝招时，在周围留下持续燃烧的灵火"; engus = "Onslaught Stake leaves lingering ghostflame on activation" },
    @{ Bnd = "item_dlc01"; Fmg = 419; Id = 9100098; zhocn = "【追踪者】发动技艺时，能使出灵火追加攻击(仅限大剑)"; engus = "Enables ghostflame Greatsword follow-ups when using Skill" },
    @{ Bnd = "item_dlc01"; Fmg = 420; Id = 9100098; zhocn = "使用技艺后，大剑能使出缠绕灵火的追加攻击"; engus = "Claw Shot allows ghostflame Greatsword follow-up attack after use" },
    # 状态界面「特殊效果」按常驻效果的文本 ID 查 menu 的 SpEffectName / Info
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 9100097; zhocn = "【追踪者】发动绝招时，能燃起灵火、蔓延周围"; engus = "[Wylder] Art activation spreads ghostflame in area" },
    @{ Bnd = "menu_dlc01"; Fmg = 215; Id = 9100097; zhocn = "遗物带来的效果"; engus = "Effect of Relic" },
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 9100098; zhocn = "【追踪者】发动技艺时，能使出灵火追加攻击(仅限大剑)"; engus = "[Wylder] Ghostflame Greatsword follow-ups when using Skill" },
    @{ Bnd = "menu_dlc01"; Fmg = 215; Id = 9100098; zhocn = "遗物带来的效果"; engus = "Effect of Relic" },
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 7020052; zhocn = "提升魔力属性攻击力"; engus = "Improved Magic Attack Power" },
    @{ Bnd = "menu_dlc01"; Fmg = 215; Id = 7020052; zhocn = "遗物带来的效果"; engus = "Effect of Relic" },
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 7020054; zhocn = "附加异常状态冻伤的效果"; engus = "Attacks Inflict Frost" },
    @{ Bnd = "menu_dlc01"; Fmg = 215; Id = 7020054; zhocn = "遗物带来的效果"; engus = "Effect of Relic" },
    # 大山羊护符（参数见 relics\src\content\bull_goat_talisman.rs）：带 From 的条目只把原文里的这一句换掉，其余保留
    @{ Bnd = "item_dlc01"; Fmg = 23; Id = 1210; From = @{ zhocn = "能提升强韧度"; engus = "Raises poise" }
       zhocn = "不会被投技与抓取以外的攻击打断"; engus = "Unflinching except against throws and grabs" },
    @{ Bnd = "item_dlc01"; Fmg = 27; Id = 1210; From = @{ zhocn = "能提升强韧度。"; engus = "Raises poise." }
       zhocn = "受到投技与抓取以外的攻击时，`n动作不会被打断。"; engus = "Actions cannot be interrupted by any attack except throws and grabs." },
    # 护符改指向的新词条 312101 的名称与说明
    @{ Bnd = "item_dlc01"; Fmg = 44; Id = 312101; zhocn = "不会被投技与抓取以外的攻击打断"; engus = "Unflinching except against throws and grabs" },
    @{ Bnd = "item_dlc01"; Fmg = 50; Id = 312101; zhocn = "不会被投技与抓取以外的攻击打断动作"; engus = "Actions cannot be interrupted except by throws and grabs" },
    # 血焰 / 黑焰 / 命定之死版的附魔 buff 名（状态界面，结构同灵火版 7020052 / 7020054）
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 7020062; zhocn = "在武器缠绕血焰"; engus = "Bloodflame Weapon" },
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 7020064; zhocn = "附加异常状态出血的效果"; engus = "Attacks Inflict Blood Loss" },
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 7020072; zhocn = "赋予武器黑焰"; engus = "Black Flame Weapon" },
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 7020082; zhocn = "赋予武器命定之死"; engus = "Destined Death Weapon" },
    @{ Bnd = "menu_dlc01"; Fmg = 214; Id = 7020084; zhocn = "提升圣属性攻击力"; engus = "Improved Holy Attack Power" },
    # 改了效果的首领球锻体：名字与说明按新效果的文本 ID 查 PermanentBuffName（51）/ Info（52），参数见 relics\src\content\permanent_buffs.rs
    @{ Bnd = "item_dlc01"; Fmg = 51; Id = 9970000; zhocn = "动作不会被打断"; engus = "Unflinching" },
    @{ Bnd = "item_dlc01"; Fmg = 52; Id = 9970000; zhocn = "受到投技与抓取以外的攻击时，`n动作不会被打断"
       engus = "Actions cannot be interrupted by any attack except throws and grabs" },
    @{ Bnd = "item_dlc01"; Fmg = 51; Id = 9970010; zhocn = "动作不会被打断，提升减伤率"; engus = "Unflinching & Improved Damage Negation" },
    @{ Bnd = "item_dlc01"; Fmg = 52; Id = 9970010; zhocn = "受到投技与抓取以外的攻击时，动作不会被打断`n并减轻受到的所有伤害"
       engus = "Actions cannot be interrupted except by throws and grabs.`nReduces all damage taken" },
    @{ Bnd = "item_dlc01"; Fmg = 51; Id = 9970030; zhocn = "提升对龙的攻击力"; engus = "Improved Damage against Dragons" },
    @{ Bnd = "item_dlc01"; Fmg = 52; Id = 9970030; zhocn = "对龙，能增加额外伤害`n古龙为 20%，飞龙等其他龙为 30%"
       engus = "Enhances attacks against dragons`n(20% against ancient dragons, 30% against other dragons)" },
    @{ Bnd = "item_dlc01"; Fmg = 51; Id = 9970040; zhocn = "提升近战攻击力"; engus = "Improved Melee Attack Power" },
    @{ Bnd = "item_dlc01"; Fmg = 52; Id = 9970040; zhocn = "提升近战攻击的攻击力`n魔法、祷告不受影响"; engus = "Raises attack power of melee attacks.`nSorceries and incantations are unaffected" },
    @{ Bnd = "item_dlc01"; Fmg = 51; Id = 9970070; zhocn = "周围陷入中毒、腐败时，获得恩惠赐福"; engus = "Poison & Rot in Vicinity Grant Blessing's Boon" },
    @{ Bnd = "item_dlc01"; Fmg = 52; Id = 9970070; zhocn = "周围人物陷入异常状态中毒、猩红腐败时`n获得恩惠赐福，缓慢恢复血量"
       engus = "Poisoning or scarlet rot in vicinity grants Blessing's Boon,`nslowly restoring HP" },
    @{ Bnd = "item_dlc01"; Fmg = 51; Id = 9970080; zhocn = "持有的卢恩越多，攻击力越高"; engus = "Attack Power Rises with Runes Held" },
    @{ Bnd = "item_dlc01"; Fmg = 52; Id = 9970080; zhocn = "根据持有卢恩的多寡，提升攻击力`n效果为「恶魔的添翼」的一半，可与其叠加"
       engus = "Enhances attack power depending on runes in possession.`nHalf the effect of Demon's Plating, stacks with it" },
    @{ Bnd = "item_dlc01"; Fmg = 51; Id = 9970090; zhocn = "死诞者被打败之后，不会复活"; engus = "Those Who Live in Death Cannot Revive" },
    @{ Bnd = "item_dlc01"; Fmg = 52; Id = 9970090; zhocn = "对死诞者，能增加额外伤害`n死诞者被打败之后，不会复活"
       engus = "Enhances attacks against Those Who Live in Death.`nThey will not revive once defeated" }
)
# 三档锻体与角色专属锻体的文本表（$TieredTexts / $TieredStatusTexts）
. (Join-Path $PSScriptRoot "tiered_texts.ps1")
if ($TieredAbilities) { $Texts += $TieredAbilityTexts }
foreach ($t in $TieredStatusTexts) {
    $Texts += @(
        @{ Bnd = "menu_dlc01"; Fmg = 214; Id = $t.Id; zhocn = $t.Zh; engus = $t.En },
        @{ Bnd = "menu_dlc01"; Fmg = 215; Id = $t.Id; zhocn = "锻体带来的效果"; engus = "Effect of Permanent Buff" }
    )
}
foreach ($t in $TieredTexts) {
    $Texts += @(
        @{ Bnd = "item_dlc01"; Fmg = 51; Id = $t.Id; zhocn = $t.Zh; engus = $t.En },
        @{ Bnd = "item_dlc01"; Fmg = 52; Id = $t.Id; zhocn = $t.ZhInfo; engus = $t.EnInfo },
        @{ Bnd = "menu_dlc01"; Fmg = 214; Id = $t.Id; zhocn = $t.Zh; engus = $t.En },
        @{ Bnd = "menu_dlc01"; Fmg = 215; Id = $t.Id; zhocn = "锻体带来的效果"; engus = "Effect of Permanent Buff" }
    )
}
foreach ($t in $TieredPassiveTexts) {
    $Texts += @(
        @{ Bnd = "item_dlc01"; Fmg = 44; Id = $t.Id; zhocn = $t.Zh; engus = $t.En },
        @{ Bnd = "item_dlc01"; Fmg = 50; Id = $t.Id; zhocn = $t.ZhInfo; engus = $t.EnInfo }
    )
}
foreach ($id in 7020062, 7020064, 7020072, 7020082, 7020084) { $Texts += @{ Bnd = "menu_dlc01"; Fmg = 215; Id = $id; zhocn = "遗物带来的效果"; engus = "Effect of Relic" } }
# 血焰 / 黑焰 / 命定之死版的词条名与常驻效果文本，句式同灵火版（参数见 relics\src\content\wylder_elements.rs）
$ElementTexts = @(
    @{ FollowUp = 7020060; Ultimate = 7010570; FollowBuff = 9100099; UltimateBuff = 9100102; Zh = "血焰"; ZhFlame = "血焰"; En = "bloodflame"; EnCap = "Bloodflame"; EnFlame = "bloodflame" },
    @{ FollowUp = 7020070; Ultimate = 7010580; FollowBuff = 9100100; UltimateBuff = 9100103; Zh = "黑焰"; ZhFlame = "黑焰"; En = "black flame"; EnCap = "Black Flame"; EnFlame = "black flame" },
    @{ FollowUp = 7020080; Ultimate = 7010590; FollowBuff = 9100101; UltimateBuff = 9100104; Zh = "命定之死"; ZhFlame = "命定之死的火焰"; En = "Destined Death"; EnCap = "Destined Death"; EnFlame = "Destined Death flames" }
)
foreach ($e in $ElementTexts) {
    $followName = "【追踪者】发动技艺时，能使出$($e.Zh)追加攻击(仅限大剑)"
    $ultimateName = "【追踪者】发动绝招时，能燃起$($e.ZhFlame)、蔓延周围"
    $Texts += @(
        @{ Bnd = "item_dlc01"; Fmg = 44; Id = $e.FollowUp
           zhocn = "【追踪者】发动技艺时，轻攻击能使出`n　缠绕$($e.Zh)的追加攻击（仅限大剑）"
           engus = "[Wylder] Standard attacks enhanced with $($e.En)`nfollow-ups when using Character Skill (greatsword only)" },
        @{ Bnd = "item_dlc01"; Fmg = 44; Id = $e.Ultimate; zhocn = $ultimateName; engus = "[Wylder] Art activation spreads $($e.EnFlame) in area" },
        @{ Bnd = "item_dlc01"; Fmg = 419; Id = $e.FollowBuff; zhocn = $followName; engus = "Enables $($e.En) Greatsword follow-ups when using Skill" },
        @{ Bnd = "item_dlc01"; Fmg = 420; Id = $e.FollowBuff; zhocn = "使用技艺后，大剑能使出缠绕$($e.Zh)的追加攻击"; engus = "Claw Shot allows $($e.En) Greatsword follow-up attack after use" },
        @{ Bnd = "item_dlc01"; Fmg = 419; Id = $e.UltimateBuff; zhocn = $ultimateName; engus = "Art activation spreads $($e.EnFlame) in area" },
        @{ Bnd = "item_dlc01"; Fmg = 420; Id = $e.UltimateBuff; zhocn = "发动绝招时，在周围留下持续燃烧的$($e.ZhFlame)"; engus = "Onslaught Stake leaves lingering $($e.EnFlame) on activation" },
        @{ Bnd = "menu_dlc01"; Fmg = 214; Id = $e.FollowBuff; zhocn = $followName; engus = "[Wylder] $($e.EnCap) Greatsword follow-ups when using Skill" },
        @{ Bnd = "menu_dlc01"; Fmg = 215; Id = $e.FollowBuff; zhocn = "遗物带来的效果"; engus = "Effect of Relic" },
        @{ Bnd = "menu_dlc01"; Fmg = 214; Id = $e.UltimateBuff; zhocn = $ultimateName; engus = "[Wylder] Art activation spreads $($e.EnFlame) in area" },
        @{ Bnd = "menu_dlc01"; Fmg = 215; Id = $e.UltimateBuff; zhocn = "遗物带来的效果"; engus = "Effect of Relic" }
    )
}

# ---- SoulsFormats 与 TAE 小工具 ----
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

function Edit-A900($Tae) {
    $tplAnim = Get-Anim $Tae $Ultimate.TemplateAnim
    $template = $tplAnim.Events | Where-Object { $_.Type -eq 2 -and (Get-Judge $_) -eq $Ultimate.Judge -and (Get-State $_) -eq $Ultimate.State } | Select-Object -First 1
    if (-not $template) { throw "relic fire event (judge $($Ultimate.Judge), state $($Ultimate.State)) not found" }

    foreach ($id in $FollowUpAnims) {
        $anim = Get-Anim $Tae $id
        $group = $anim.EventGroups | Where-Object { $_.GroupType -eq 2 } | Select-Object -First 1
        if (-not $group) { $group = $template.Group.GetClone(); $anim.EventGroups.Add($group) }
        $made = 0
        foreach ($r in $FollowUpReplace) {
            $old = $anim.Events | Where-Object { $_.Type -eq $r.Type -and $r.Fire.ContainsKey([BitConverter]::ToInt32((Get-Params $_), 0)) } | Select-Object -First 1
            if (-not $old) { throw "anim ${id}: event type $($r.Type) value $($r.Fire.Keys -join '/') not found" }
            $at = $anim.Events.IndexOf($old)
            $fire = @{ Judge = $r.Fire[[BitConverter]::ToInt32((Get-Params $old), 0)]; State = 2027; Delay = 0 }
            foreach ($e in @($fire) + $r.Events) {
                $anim.Events.Insert($at, (New-BulletEvent $template $e.Judge $e.State ($old.StartTime + $e.Delay * $Frame) $group))
                $at++; $made++
            }
            Remove-Event $anim $old
        }
        "  追加攻击动画 ${id}：无条件的附魔 / 刀身特效事件 $($FollowUpReplace.Count) 个 → 条件子弹事件 x$made"
    }
    foreach ($anim in $Tae.Animations) {
        $fires = @($anim.Events | Where-Object { $_.Type -eq 2 -and (Get-Judge $_) -eq $Ultimate.Judge -and (Get-State $_) -eq $Ultimate.State })
        foreach ($fire in $fires) {
            $at = $anim.Events.IndexOf($fire)
            foreach ($c in $Ultimate.Copies) {
                $at++
                $anim.Events.Insert($at, (New-BulletEvent $fire $c.Judge $c.State $fire.StartTime $fire.Group))
            }
        }
        if ($fires.Count) { "  绝招动画 $($anim.ID)：遗物地火事件 x$($fires.Count) 各复制 $($Ultimate.Copies.Count) 种元素版" }
    }
    Edit-FollowUpArmor $Tae
    Edit-Blast $Tae
}

# 跳转表事件参数 +0 为跳转表 ID；原版每种跳转表 ID 各占一个事件组，新事件也单独建组
function Edit-FollowUpArmor($Tae) {
    $tpl = (Get-Anim $Tae $FollowUpArmor.TemplateAnim).Events |
        Where-Object { $_.Type -eq 0 -and [BitConverter]::ToInt32((Get-Params $_), 0) -eq $FollowUpArmor.JumpTable } | Select-Object -First 1
    if (-not $tpl) { throw "jump table $($FollowUpArmor.JumpTable) not found in anim $($FollowUpArmor.TemplateAnim)" }
    foreach ($id in $FollowUpAnims) {
        $anim = Get-Anim $Tae $id
        $win = $anim.Events | Where-Object { $_.Type -eq $FollowUpArmor.WindowEvent } | Select-Object -First 1
        if (-not $win) { throw "anim ${id}: window event type $($FollowUpArmor.WindowEvent) not found" }
        $ev = $tpl.GetClone($false)
        $ev.StartTime = $win.StartTime
        $ev.EndTime = $win.EndTime
        $ev.Group = New-Group $anim $tpl
        $anim.Events.Insert($anim.Events.IndexOf($win), $ev)
        "  追加攻击动画 {0}：霸体（跳转表 {1}）{2:F3}~{3:F3}" -f $id, $FollowUpArmor.JumpTable, $ev.StartTime, $ev.EndTime
    }
}

function Edit-Blast($Tae) {
    $inv = [Globalization.CultureInfo]::InvariantCulture
    foreach ($id in $Blast.Anims) {
        $anim = Get-Anim $Tae $id
        $src = $anim.Events | Where-Object { $_.Type -eq 67 -and [BitConverter]::ToInt32((Get-Params $_), 0) -eq $Blast.MarkFrom } | Select-Object -First 1
        if (-not $src) { throw "anim ${id}: mark anchor event $($Blast.MarkFrom) not found" }
        $mark = $src.GetClone($false)
        $p = Get-Params $src
        [BitConverter]::GetBytes([int]$Blast.RegularMark).CopyTo($p, 0)
        $mark.SetParameterBytes($false, $p, $false)
        $mark.EndTime = $mark.StartTime + $Frame
        $mark.Group = New-Group $anim $src
        $anim.Events.Insert($anim.Events.IndexOf($src) + 1, $mark)

        $blasts = @($anim.Events | Where-Object { $_.Type -eq 2 -and $Blast.Judges.ContainsKey((Get-Judge $_)) -and (Get-State $_) -eq 0 })
        if (-not $blasts.Count) { throw "anim ${id}: blast event not found" }
        foreach ($b in $blasts) {
            Set-State $b $Blast.RegularState
            $ghost = New-BulletEvent $b $Blast.Judges[(Get-Judge $b)] $Blast.GhostState $b.StartTime (New-Group $anim $b)
            $anim.Events.Insert($anim.Events.IndexOf($b) + 1, $ghost)
        }
        $self = $anim.Events | Where-Object { $_.Type -eq 2 -and (Get-Judge $_) -eq $Blast.AfterfireTemplateJudge } | Select-Object -First 1
        if (-not $self) { throw "anim ${id}: afterfire template event (judge $($Blast.AfterfireTemplateJudge)) not found" }
        $fxs = @($anim.Events | Where-Object { $_.Type -eq 96 -and [BitConverter]::ToInt32((Get-Params $_), 0) -eq $Blast.AfterfireFx })
        foreach ($fx in $fxs) {
            $len = [string]::Format($inv, "{0:F3}", $fx.EndTime - $fx.StartTime)
            $judge = $Blast.AfterfireJudges[$len]
            if (-not $judge) { throw "anim ${id}: no afterfire judge for duration $len" }
            $ev = New-BulletEvent $self $judge $Blast.RegularState ($fx.StartTime + $Frame) (New-Group $anim $self)
            $anim.Events.Insert($anim.Events.IndexOf($fx), $ev)
            Remove-Event $anim $fx
        }
        "  绝招动画 ${id}：爆炸 x$($blasts.Count) 改条件并加灵火版，余火特效 x$($fxs.Count) 改条件"
    }
}

# HKS 字节码（\x1bLua 开头）的数字常量是「类型 3 + 大端 float32」，源码文本则直接是 env(GetSpEffectID, ID)；
# 两种写法都要求恰好命中 $HksGate.Count 处，返回改好的字节
function Edit-HksGate([byte[]]$Bytes) {
    $latin = [Text.Encoding]::GetEncoding(28591)
    $isBytecode = $Bytes.Length -gt 4 -and $Bytes[0] -eq 0x1B -and $latin.GetString($Bytes, 1, 3) -eq "Lua"
    if ($isBytecode) {
        $be = { param($v) $b = [BitConverter]::GetBytes([single]$v); [Array]::Reverse($b); $latin.GetString([byte[]](@(3) + $b)) }
        $from = & $be $HksGate.From; $to = & $be $HksGate.To
    } else {
        $from = "env(GetSpEffectID, $($HksGate.From))"; $to = "env(GetSpEffectID, $($HksGate.To))"
    }
    $text = $latin.GetString($Bytes)
    $n = ([regex]::Matches($text, [regex]::Escape($from))).Count
    if ($n -ne $HksGate.Count) { throw "c0000.hks: expected $($HksGate.Count) follow-up checks, found $n" }
    $kind = if ($isBytecode) { "字节码常量" } else { "源码" }
    "c0000.hks: 追加攻击入口 $($HksGate.From) → $($HksGate.To) x$n（$kind）" | Out-Host
    , $latin.GetBytes($text.Replace($from, $to))
}

# ---- 1. 动作文件 ----
$aniOut = Join-Path $OutDir "chr\c0000.anibnd.dcx"
$bnd = Read-Bnd (Resolve-Path $AnibndSrc).Path
$taeFile = Find-File $bnd "\a900.tae"
$tae = [SoulsFormats.TAE]::Read($taeFile.Bytes.ToArray())
"a900.tae:"
Edit-A900 $tae
$taeFile.Bytes = [byte[]]$tae.Write()
Write-Bnd $bnd $aniOut

# ---- 2. HKS ----
$hksOut = Join-Path $OutDir "action\script\c0000.hks"
New-Item -ItemType Directory -Force (Split-Path $hksOut) | Out-Null
[IO.File]::WriteAllBytes($hksOut, (Edit-HksGate ([IO.File]::ReadAllBytes((Resolve-Path $HksSrc).Path))))

# ---- 3. 文本 ----
foreach ($lang in "zhocn", "engus") {
    foreach ($bndName in ($Texts | ForEach-Object { $_.Bnd } | Sort-Object -Unique)) {
        $src = Join-Path $MsgSrcDir "$lang\$bndName.msgbnd.dcx"
        $msg = Read-Bnd $src
        $added = 0; $replaced = 0
        foreach ($group in ($Texts | Where-Object { $_.Bnd -eq $bndName } | Group-Object { $_.Fmg })) {
            $file = $msg.Files | Where-Object { $_.ID -eq [int]$group.Name } | Select-Object -First 1
            if (-not $file) { throw "$lang\$bndName FMG $($group.Name) not found" }
            $fmg = [SoulsFormats.FMG]::Read($file.Bytes.ToArray())
            foreach ($t in $group.Group) {
                $value = $t[$lang]
                $exist = $fmg.Entries | Where-Object { $_.ID -eq $t.Id } | Select-Object -First 1
                if ($t.From) {
                    $from = $t.From[$lang]
                    if (-not $exist) { throw "$lang\$bndName FMG $($group.Name) entry $($t.Id) not found" }
                    $n = ([regex]::Matches($exist.Text, [regex]::Escape($from))).Count
                    if ($n -ne 1) { throw "$lang\$bndName FMG $($group.Name) entry $($t.Id): expected '$from' once, found $n" }
                    $exist.Text = $exist.Text.Replace($from, $value); $replaced++
                    continue
                }
                if ($exist -and $exist.Text -ne $value) { throw "$lang\$bndName FMG $($group.Name) entry $($t.Id) already used: $($exist.Text)" }
                if (-not $exist) { $fmg.Entries.Add((New-Object SoulsFormats.FMG+Entry($fmg, $t.Id, $value))); $added++ }
            }
            $sorted = @($fmg.Entries | Sort-Object ID)
            $fmg.Entries.Clear()
            foreach ($e in $sorted) { $fmg.Entries.Add($e) }
            $file.Bytes = [byte[]]$fmg.Write()
        }
        Write-Bnd $msg (Join-Path $OutDir "msg\$lang\$bndName.msgbnd.dcx")
        "$lang\$bndName.msgbnd.dcx: 新增文本 $added 条，替换 $replaced 条"
    }
}
