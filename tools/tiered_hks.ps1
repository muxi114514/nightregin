<#
  三档锻体的动作脚本部分（只有 MMV：c0000.hks 是源码文本）。把 tools\tiered_buffs.lua 与 tiered_roles.lua 插在 Update 之前，再接上几处调用：
    Update                  每帧调用 TieredUpdate（放绝招的触发、圣杯瓶为 0 时减伤、守护者强化防御、多效果锻体的子效果）
    GetAttackRequest        原函数改名为 TieredGetAttackRequestBase，由 lua 里的同名包装拦下金10 的绝招、金11 的技艺
    AddStamina              原函数改名为 TieredAddStaminaBase，lua 里的同名包装给紫2 的翻滚 / 后跳精力打折
    ExecEvent               原函数改名为 TieredExecEventBase，lua 里的同名包装按动作事件挂效果（武器词条战技 / 施法霸体、无赖1 / 5）
    ExecAttack 技艺分支      地面与空中（追踪者、铁之眼）放技艺时调用 TieredOnSkill
    ExecDejavu / ExecTrance 女爵、送葬者的技艺不走 ExecAttack，在开始处拦截并调用 TieredOnSkill
    第七批角色             守护者战步的条件加「没有守护者9」；三处妖刀完美弹反判定改调 TieredDemonJustGuard（执行者2 / 3）；
                           学者受击爆发也认屏障副本（学者6）
    倒地                    IsDirectDeath 与 NearDeathCommonFunction 的安定者复活判断也认复仇者5 借出的次数 9971493
    执行者8                 妖刀的技艺改进纳刀架势（TieredEnterStance）；架势借月隐的战技，不看 FP、不预扣；
                           绝招先问 TieredCrucibleUlt（熔炉形态，tools\tiered_crucible.lua），形态期间疾跑 / L3 / 非战斗区不收刀、不禁疾跑
  就地改写 -Hks（relic_assets.ps1 生成到 pkg 的文件），每处命中次数都要对得上，MMV 更新后对不上会直接报错
#>
param([Parameter(Mandatory)] [string]$Hks)
$ErrorActionPreference = "Stop"
$path = (Resolve-Path $Hks).Path
$utf8 = New-Object Text.UTF8Encoding($false)
$text = [IO.File]::ReadAllText($path, $utf8)
if ($text.Contains("TieredUpdate")) { throw "$path already has the tiered buff hooks" }

# 按行首缩进匹配一行（默认恰好一处），$Insert 收到缩进返回要替换成的文本
function Edit-Line([string]$Line, [string]$What, [scriptblock]$Insert, [int]$Count = 1) {
    $re = "(?m)^([ \t]*)" + [regex]::Escape($Line) + "(\r?)$"
    $n = ([regex]::Matches($script:text, $re)).Count
    if ($n -ne $Count) { throw "c0000.hks: expected $Count '$Line' ($What), found $n" }
    $script:text = [regex]::Replace($script:text, $re, { param($m) & $Insert $m.Groups[1].Value $m.Groups[2].Value })
    "c0000.hks: $What"
}
# 同一行在文件里有多处时，按紧跟的下一行区分（恰好一处），只改第一行
function Edit-Pair([string]$Line, [string]$Next, [string]$What, [scriptblock]$Insert) {
    $re = "(?m)^([ \t]*)" + [regex]::Escape($Line) + "(\r?)(?=\n[ \t]*" + [regex]::Escape($Next) + "\r?$)"
    $n = ([regex]::Matches($script:text, $re)).Count
    if ($n -ne 1) { throw "c0000.hks: expected 1 '$Line' before '$Next' ($What), found $n" }
    $script:text = [regex]::Replace($script:text, $re, { param($m) & $Insert $m.Groups[1].Value $m.Groups[2].Value })
    "c0000.hks: $What"
}
# 同上，按紧挨的上一行区分
function Edit-After([string]$Prev, [string]$Line, [string]$What, [scriptblock]$Insert) {
    $re = "(?m)(?<=^[ \t]*" + [regex]::Escape($Prev) + "\r?\n)([ \t]*)" + [regex]::Escape($Line) + "(\r?)$"
    $n = ([regex]::Matches($script:text, $re)).Count
    if ($n -ne 1) { throw "c0000.hks: expected 1 '$Line' after '$Prev' ($What), found $n" }
    $script:text = [regex]::Replace($script:text, $re, { param($m) & $Insert $m.Groups[1].Value $m.Groups[2].Value })
    "c0000.hks: $What"
}

Edit-Line "ExecUltCharge()" "Update 每帧调用 TieredUpdate" { param($i, $cr) "${i}ExecUltCharge()$cr`n${i}TieredUpdate()$cr" }
Edit-Line "function GetAttackRequest(is_guard, blend_type)" "GetAttackRequest 改名，外面包一层" { param($i, $cr) "${i}function TieredGetAttackRequestBase(is_guard, blend_type)$cr" }
Edit-Line "function AddStamina(num)" "AddStamina 改名，外面包一层（紫2 闪避精力打折）" { param($i, $cr) "${i}function TieredAddStaminaBase(num)$cr" }
Edit-Line "function ExecEvent(state)" "ExecEvent 改名，外面包一层（武器词条战技 / 施法霸体）" { param($i, $cr) "${i}function TieredExecEventBase(state)$cr" }
Edit-Line "elseif request == ATTACK_REQUEST_SKILL then" "地面技艺" { param($i, $cr) "${i}elseif request == ATTACK_REQUEST_SKILL then$cr`n${i}    TieredOnSkill()$cr" }
Edit-Line "if request == ATTACK_REQUEST_SKILL then" "空中技艺（只有追踪者、铁之眼放得出）" {
    param($i, $cr) "${i}if request == ATTACK_REQUEST_SKILL then$cr`n${i}    if env(GetHeroID) == HERO_BALANCE or env(GetHeroID) == HERO_SHOOTER then TieredOnSkill() end$cr"
}
Edit-Line 'SetVariable("AddDejavuBlend", 1)' "女爵技艺" {
    param($i, $cr) "${i}if TieredSkillBlocked() then return FALSE end$cr`n${i}TieredOnSkill()$cr`n${i}SetVariable(`"AddDejavuBlend`", 1)$cr"
}
Edit-Line "ISENABLE_TRANCEEND = FALSE" "送葬者技艺" {
    param($i, $cr) "${i}if TieredSkillBlocked() then return FALSE end$cr`n${i}TieredOnSkill()$cr`n${i}ISENABLE_TRANCEEND = FALSE$cr"
}
# 第七批角色（效果 ID 见 relics\src\tiered\role_actions.rs）
Edit-Line "elseif env(GetHeroID) == HERO_TANK " "守护者9：带锻体时不选战步" {
    param($i, $cr) "${i}elseif env(GetHeroID) == HERO_TANK and env(GetSpEffectID, 9971740) == FALSE $cr"
}
Edit-Line "if damage_type == DAMAGE_TYPE_GUARDBREAK and IsDemonSwordMode() == TRUE and env(GetSpEffectID, 707000) == TRUE then" "妖刀完美弹反被破防（执行者2 / 3）" {
    param($i, $cr) "${i}if damage_type == DAMAGE_TYPE_GUARDBREAK and IsDemonSwordMode() == TRUE and TieredDemonJustGuard() == TRUE then$cr"
}
Edit-Line "if env(GetSpEffectID, 707000) == TRUE then" "妖刀完美弹反（执行者2 / 3）" { param($i, $cr) "${i}if TieredDemonJustGuard() == TRUE then$cr" } 2
Edit-Line "if env(GetSpEffectID, 708050) == TRUE and is_damaged == TRUE then" "学者屏障受击爆发（学者6）" {
    param($i, $cr) "${i}if (env(GetSpEffectID, 708050) == TRUE or TieredHas(9971776)) and is_damaged == TRUE then$cr"
}
# 复仇者5 借出的复活次数：单人有次数时不直接死，倒地时走自行复活（效果 ID 见 relics\src\tiered\aura.rs）
Edit-Line "and env(GetSpEffectID, 6999500) == FALSE and env(GetSpEffectID, 8970061) == FALSE then" "单人打夜晚首领倒地：有复活次数不直接死（复仇者5）" {
    param($i, $cr) "${i}and env(GetSpEffectID, 6999500) == FALSE and env(GetSpEffectID, 8970061) == FALSE and not TieredHas(9971493) then$cr"
}
Edit-Line "elseif env(GetSpEffectID, 8970061) == FALSE then" "单人倒地：有复活次数不直接死（复仇者5）" {
    param($i, $cr) "${i}elseif env(GetSpEffectID, 8970061) == FALSE and not TieredHas(9971493) then$cr"
}
Edit-Line "or env(GetSpEffectID, 8970061) == TRUE then" "倒地自行复活也认复活次数（复仇者5）" {
    param($i, $cr) "${i}or env(GetSpEffectID, 8970061) == TRUE or TieredHas(9971493) then$cr"
}
Edit-Line "if env(GetSpEffectID, 8970061) == TRUE then" "起身动作选自行复活，并用掉复活次数（复仇者5）" { param($i, $cr) "${i}if TieredIsAutoRevival() == TRUE then$cr" }
# 执行者8
Edit-Line "c_SwordArtsID = art_id" "拔刀时脚本的战技编号当作月隐（执行者8）" { param($i, $cr) "${i}c_SwordArtsID = TieredStanceArtId(art_id)$cr" }
Edit-Line "elseif request == ATTACK_REQUEST_DEMONSWORDARTS then" "妖刀技艺改进纳刀架势（执行者8）" {
    param($i, $cr) "${i}elseif request == ATTACK_REQUEST_DEMONSWORDARTS then$cr`n${i}    if TieredEnterStance(blend_type) == TRUE then return TRUE end$cr"
}
Edit-Line "function SetSwordArtsPointInfo(button, is_point_consume, to_state_event)" "妖刀架势不预扣 FP（执行者8）" {
    param($i, $cr) "${i}function SetSwordArtsPointInfo(button, is_point_consume, to_state_event)$cr`n${i}    if TieredCrucibleDrawn() == TRUE then is_point_consume = FALSE end$cr"
}
Edit-Line "if artsID == 92 or artsID == 93 or artsID == 94 or artsID == 112 or artsID == 17 then" "妖刀架势不看 FP（执行者8）" {
    param($i, $cr) "${i}if artsID == 92 or artsID == 93 or artsID == 94 or artsID == 112 or artsID == 17 or TieredCrucibleDrawn() == TRUE then$cr"
}
# 执行者8 绝招：带锻体时变熔炉形态，形态期间妖刀不收
Edit-Line 'ExecEventAllBody("W_DemonBeastStart")' "执行者绝招：带锻体时改熔炉形态（执行者8）" {
    param($i, $cr) "${i}if TieredCrucibleUlt(blend_type) == FALSE then$cr`n${i}    ExecEventAllBody(`"W_DemonBeastStart`")$cr`n${i}end$cr"
}
Edit-Line "if IsDemonSwordMode() == TRUE and env(ActionDuration, ACTION_ARM_L3) > 0 then" "熔炉形态 L3 不收刀（执行者8）" {
    param($i, $cr) "${i}if IsDemonSwordMode() == TRUE and env(ActionDuration, ACTION_ARM_L3) > 0 and not TieredCrucibleForm() then$cr"
}
Edit-Pair "if IsDemonSwordMode() == TRUE then" "ISENABLE_DEMONSWORDSTANCE_TO_SPRINTSTART = TRUE" "熔炉形态疾跑不收刀（执行者8）" {
    param($i, $cr) "${i}if IsDemonSwordMode() == TRUE and not TieredCrucibleForm() then$cr"
}
Edit-Pair "if IsDemonSwordMode() == TRUE then" "ExecEventHalfBlend(Event_DemonSwordStanceEnd, blend_type)" "熔炉形态进非战斗区不收刀（执行者8）" {
    param($i, $cr) "${i}if IsDemonSwordMode() == TRUE and not TieredCrucibleForm() then$cr"
}
Edit-Line "function AttackBothLight6_onUpdate()" "熔炉形态踏地播完拔刀（执行者8）" {
    param($i, $cr) "${i}function AttackBothLight6_onUpdate()$cr`n${i}    if TieredCrucibleStompUpdate() == TRUE then return end$cr"
}
Edit-Line "function AttackBothHeavy1SubStart_onUpdate()" "熔炉形态普通之尾不转去重攻击起手（执行者8）" {
    param($i, $cr) "${i}function AttackBothHeavy1SubStart_onUpdate()$cr`n${i}    if TieredCrucibleArtUpdate() == TRUE then return end$cr"
}
# 拔刀时每帧把速度档压回 2、也不切到疾跑档 3，熔炉形态放开（否则跑一段就被打回）
Edit-Pair "if IsDemonSwordMode() == TRUE then" "BanSprint()" "熔炉形态不禁疾跑（执行者8）" {
    param($i, $cr) "${i}if IsDemonSwordMode() == TRUE and not TieredCrucibleForm() then$cr"
}
Edit-Line "elseif env(GetSpEffectID, 100010) == TRUE and IsDemonSwordMode() == FALSE then" "熔炉形态能切疾跑速度（执行者8）" {
    param($i, $cr) "${i}elseif env(GetSpEffectID, 100010) == TRUE and (IsDemonSwordMode() == FALSE or TieredCrucibleForm()) then$cr"
}
# 拔刀的架势移动没有疾跑动作：熔炉形态疾跑时走普通移动；MMV 的疾跑自动取消看上半身节点，拔刀时会误判，移动中不取消
Edit-After "-- Executor - lower body for Skill Cursed Sword" "elseif IsDemonSwordMode() == TRUE then" "熔炉形态疾跑用普通移动（执行者8）" {
    param($i, $cr) "${i}elseif IsDemonSwordMode() == TRUE and not TieredCrucibleSprint() then$cr"
}
Edit-Line 'and IsNodeActive("Move_Upper Selector") == FALSE ' "熔炉形态移动中不取消疾跑（执行者8）" {
    param($i, $cr) "${i}and IsNodeActive(`"Move_Upper Selector`") == FALSE and not TieredCrucibleMoving() $cr"
}
# 最后插函数本体（里面的包装与原函数同名，先插会让上面的改名匹配到两处）
$lua = (("tiered_buffs.lua", "tiered_roles.lua", "tiered_crucible.lua" | ForEach-Object { [IO.File]::ReadAllText((Join-Path $PSScriptRoot $_), $utf8) }) -join "`n") -replace "\r?\n", "`r`n"
Edit-Line "function Update()" "插入三档锻体函数" { param($i, $cr) "$lua" + "function Update()$cr" }
[IO.File]::WriteAllText($path, $text, $utf8)
