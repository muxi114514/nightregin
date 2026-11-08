-- ==== GoldenAge 阎魔刀（tools\ga_hks.ps1 插入）====
-- 招式都走普通攻击状态：这里按武器特殊类别 73 决定下一招，动作与判定在招式表 a73 / a373（tools\ga_moveset.ps1）。
-- 现在接了连段、L2 入口、幻影剑（L1）、魔人化（切换单双持）、次元斩绝（多才多艺满层或世界之眼，按住 L2 再按 L1）

YAMATO_CATEGORY = 73
-- 脚本已经放出招式时 GetAttackRequest 的返回值，ExecAttack 见到它只做收尾
ATTACK_REQUEST_YAMATO = 9973
YAMATO_HEAVY = "W_AttackRightHeavySpecial1Start"
YAMATO_BOTH1 = "W_AttackBothLight1"
YAMATO_DUAL1 = "W_AttackDualLight1"
-- 行为图没有法环的潜行攻击状态，这两招借现成状态播放（tools\ga_moveset.ps1 把动作搬过去）：
-- ComboB-2（魔人）法环 W_AttackBothLightStealth，空中次元斩结束 法环 W_AttackDualStealth
YAMATO_COMBO_B2 = "W_AttackRightLightCounter"
YAMATO_AIR_SLASH_END = "W_AttackRightSpecial1"
-- 次元斩绝：GA 只判断冷却效果 10020083，没有地方挂它，这里放招时挂上（持续 30 秒）
YAMATO_FINISHER = "W_AttackDualBackStep"
YAMATO_FINISHER_COOLDOWN = 10020083
-- 招式事件挂的窗口效果：5030 / 5035 派生，5033 / 5034 / 5038 转下一段；10020084 / 10020085 记下按键时机（普通 / 完美）
YAMATO_WINDOW_DERIVE = 5030
YAMATO_WINDOW_STEP = 5035
YAMATO_WINDOW_NEXT = 5033
YAMATO_WINDOW_LOOP = 5034
YAMATO_WINDOW_END = 5038
YAMATO_TIMING_NORMAL = 10020084
YAMATO_TIMING_PERFECT = 10020085
YAMATO_DEVIL = 10020081
-- 解除动作挂上它时清掉魔人（法环放在 common.emevd 事件里，这里放进 Update，不用动事件脚本）
YAMATO_DEVIL_END = 10020087
YAMATO_DEVIL_ENTER = "W_AttackBothSpecial1"
YAMATO_DEVIL_LEAVE = "W_AttackBothSpecial2"
YAMATO_BACK_ANGLE = 160
YAMATO_FRONT_ANGLE = 45
YAMATO_SIDE_ANGLE = 90
YAMATO_REAR_ANGLE = 135
-- 幻影剑：冷却 0.2 秒；A / B 记下一发走哪边（A 是武器常驻效果）；SHOT 发一把，分别在假点 880 / 881；后四个是方向阵式
YAMATO_MIRAGE_COOLDOWN = 10020074
YAMATO_MIRAGE_SIDE_A = 10020070
YAMATO_MIRAGE_SIDE_B = 10020071
YAMATO_MIRAGE_SHOT_A = 10020072
YAMATO_MIRAGE_SHOT_B = 10020073
YAMATO_MIRAGE_FRONT = 10020090
YAMATO_MIRAGE_LEFT = 10020091
YAMATO_MIRAGE_RIGHT = 10020092
YAMATO_MIRAGE_BACK = 10020093

-- 状态的下一招。b1 / b2 双手轻 / 重，r1 / r2 / l1 同名参数；dash：按住 L2 且有派生窗口时冲刺
-- always：动作只有一条路（没有后段复位的那次调用）；step / devil：有派生窗口 5035 / 魔人时 b1 换成这招
YAMATO_COMBO = {
    AttackRightLight2 = { b1 = "W_AttackRightLight3", b2 = YAMATO_HEAVY, dash = TRUE },
    AttackRightLight3 = { b1 = "W_AttackRightLight4", b2 = YAMATO_HEAVY, dash = TRUE, step = "W_AttackRightLight5" },
    AttackRightLight4 = { b1 = "W_AttackRightLight2", b2 = YAMATO_HEAVY, dash = TRUE },
    AttackRightLight5 = { b1 = "W_AttackRightLight6", b2 = YAMATO_HEAVY, dash = TRUE },
    AttackRightLight6 = { b1 = "W_AttackRightLight2", b2 = YAMATO_HEAVY, dash = TRUE, always = TRUE },
    AttackDualLight2 = { b1 = "W_AttackBothDash" },
    AttackDualLight3 = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1 },
    AttackDualLight5 = { b1 = "W_AttackBothDash" },
    AttackDualLight6 = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1 },
    AttackDualBackStep = { r1 = "W_AttackBothHeavyCounter", b1 = "W_AttackBothHeavyCounter", b2 = YAMATO_DUAL1, l1 = "W_AttackRightLight1" },
    AttackBothLight1 = { b1 = "W_AttackBothLight2", b2 = YAMATO_DUAL1 },
    AttackBothLight2 = { b1 = "W_AttackBothLight3", b2 = YAMATO_DUAL1, step = "W_AttackBothLightStep" },
    AttackBothLight3 = { b1 = "W_AttackBothLight4", b2 = YAMATO_DUAL1, step = "W_AttackBothHeavy1Start" },
    AttackBothLight4 = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1, devil = "W_AttackBothLight5" },
    AttackBothLight5 = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1, devil = "W_AttackBothLight6" },
    AttackBothLight6 = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1, always = TRUE },
    AttackBothHeavy2Start = { r1 = YAMATO_BOTH1, b1 = YAMATO_BOTH1, r2 = "W_AttackRightHeavy1Start", l1 = "W_AttackRightLight1", always = TRUE },
    AttackBothHeavySpecial2End = { r1 = YAMATO_BOTH1, b1 = YAMATO_BOTH1, r2 = "W_AttackRightHeavy1Start", l1 = "W_AttackRightLight1", always = TRUE },
    AttackBothSpecial1 = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1, always = TRUE },
    AttackBothSpecial2 = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1, always = TRUE },
    AttackBothDash = { b1 = "W_AttackRightLight2", b2 = YAMATO_HEAVY, back = YAMATO_HEAVY },
    AttackBothHeavyDash = { r1 = YAMATO_BOTH1, b1 = YAMATO_BOTH1, r2 = "W_AttackRightHeavy1Start", l1 = "W_AttackRightLight1" },
    AttackBothLightStep = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1, devil = YAMATO_COMBO_B2 },
    AttackBothBackstep = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1, devil = YAMATO_COMBO_B2 },
    AttackRightLightCounter = { b1 = YAMATO_BOTH1, b2 = YAMATO_DUAL1 },
    AttackRightSpecial1 = { b1 = "W_AttackRightLight2", b2 = YAMATO_HEAVY, always = TRUE },
    AttackRightHeavyCounter = { r1 = YAMATO_BOTH1, b1 = YAMATO_BOTH1, r2 = "W_AttackRightHeavy1Start", l1 = "W_AttackRightLight1" },
    AttackBothHeavyCounter = { b1 = "W_AttackBothLight2", b2 = YAMATO_DUAL1 }
}
-- 当前状态这一帧要用的下一招（combo 字段），AttackCommonFunction 用掉
-- 必须是表字段：脚本末尾给 _G 设了元表，读没有值的全局变量得到 dummy 函数而不是 nil
YAMATO_FRAME = {}

function YamatoEquipped()
    return env(GetEquipWeaponSpecialCategoryNumber, HAND_RIGHT) == YAMATO_CATEGORY
end

function YamatoHas(id)
    return env(GetSpEffectID, id) == TRUE
end

function YamatoFire(event)
    ExecEventAllBody(event)
    return TRUE
end

function YamatoBackInput()
    return YAMATO_BACK_ANGLE <= math.abs(GetVariable("MoveAngle"))
end

-- 重攻击派生：窗口里松开转第二段，按住继续蓄
function YamatoHeavyCharge()
    if YamatoHas(YAMATO_WINDOW_NEXT) then
        if env(ActionDuration, ACTION_ARM_R1) <= 0 or env(ActionCancelRequest, ACTION_ARM_R1) == TRUE then
            return YamatoFire("W_AttackBothHeavy2Start")
        end
        return YamatoFire("W_AttackBothHeavy1End")
    end
    return FALSE
end

-- 第二段收招：松开 L2 回到第二段，按攻击键冲刺，按住 L2 继续
function YamatoHeavyRelease()
    if not YamatoHas(YAMATO_WINDOW_NEXT) then
        return FALSE
    end
    if env(ActionDuration, ACTION_ARM_L2) <= 0 or env(ActionCancelRequest, ACTION_ARM_L2) == TRUE then
        return YamatoFire("W_AttackBothHeavy2Start")
    end
    if env(ActionDuration, ACTION_ARM_R2) > 0 or env(ActionDuration, ACTION_ARM_R1) > 0 or env(ActionDuration, ACTION_ARM_L1) > 0 then
        return YamatoFire("W_AttackBothDash")
    end
    return YamatoFire("W_AttackBothHeavy2End")
end

-- 窗口里按键记下时机，收招窗口按时机选下一招
function YamatoRecordTiming(pressed)
    if YamatoHas(YAMATO_WINDOW_NEXT) and pressed then
        act(AddSpEffect, YAMATO_TIMING_NORMAL)
    elseif YamatoHas(YAMATO_WINDOW_STEP) and pressed then
        act(AddSpEffect, YAMATO_TIMING_PERFECT)
    end
end

-- 空中次元斩类：窗口里没按键就收招
function YamatoDashChain()
    YamatoRecordTiming(env(ActionDuration, ACTION_ARM_L2) > 0)
    if YamatoHas(YAMATO_WINDOW_END) then
        if YamatoHas(YAMATO_TIMING_NORMAL) then
            return YamatoFire("W_AttackDualDash")
        elseif YamatoHas(YAMATO_TIMING_PERFECT) then
            return YamatoFire("W_AttackDualRolling")
        end
        return YamatoFire(YAMATO_AIR_SLASH_END)
    end
    return FALSE
end

-- 次元斩类
function YamatoSkillChain()
    YamatoRecordTiming(env(ActionCancelRequest, ACTION_ARM_L2) == TRUE)
    if YamatoHas(YAMATO_WINDOW_END) then
        if YamatoHas(YAMATO_TIMING_NORMAL) then
            return YamatoFire("W_AttackBothHeavySpecial1End")
        elseif YamatoHas(YAMATO_TIMING_PERFECT) then
            return YamatoFire("W_AttackBothHeavySpecial2Start")
        end
        return YamatoFire("W_AttackBothHeavySpecial2End")
    end
    return FALSE
end

function YamatoWindowTo(window, event)
    if YamatoHas(window) then
        return YamatoFire(event)
    end
    return FALSE
end

-- 状态的直接转段，放出去了就不再走原函数
YAMATO_TRANSITION = {
    AttackRightHeavySpecial1Start = function()
        return YamatoWindowTo(YAMATO_WINDOW_NEXT, "W_AttackRightHeavySpecial1End")
    end,
    AttackRightHeavySpecial1End = function()
        if env(IsLanding) == TRUE then
            return YamatoFire("W_AttackRightHeavySpecial2Start")
        end
        return YamatoWindowTo(YAMATO_WINDOW_LOOP, "W_AttackRightHeavySpecial1End")
    end,
    AttackDualLight1 = function()
        return YamatoWindowTo(YAMATO_WINDOW_LOOP, "W_AttackDualLight2")
    end,
    AttackDualLight2 = function()
        return YamatoWindowTo(YAMATO_WINDOW_NEXT, "W_AttackDualLight3")
    end,
    AttackDualLight4 = function()
        return YamatoWindowTo(YAMATO_WINDOW_LOOP, "W_AttackDualLight5")
    end,
    AttackDualLight5 = function()
        if env(ActionDuration, ACTION_ARM_L2) == 0 then
            return YamatoFire("W_AttackDualLight6")
        end
        return YamatoWindowTo(YAMATO_WINDOW_NEXT, "W_AttackDualLight5")
    end,
    AttackDualDash = YamatoDashChain,
    AttackDualRolling = YamatoDashChain,
    AttackBothHeavy1Start = YamatoHeavyCharge,
    AttackBothHeavy1End = YamatoHeavyCharge,
    AttackBothHeavy2End = YamatoHeavyRelease,
    AttackBothHeavySpecial1Start = YamatoSkillChain,
    AttackBothHeavySpecial1End = YamatoSkillChain,
    AttackBothHeavySpecial2Start = YamatoSkillChain
}

-- 每个攻击状态 onUpdate 开头调用，返回 TRUE 表示已转段，原函数不用再走
function YamatoUpdate(state)
    YAMATO_FRAME.combo = nil
    if not YamatoEquipped() then
        return FALSE
    end
    local transition = YAMATO_TRANSITION[state]
    if transition ~= nil and transition() == TRUE then
        return TRUE
    end
    local combo = YAMATO_COMBO[state]
    if combo == nil then
        return FALSE
    end
    if combo.dash == TRUE and env(ActionDuration, ACTION_ARM_L2) > 0 and YamatoHas(YAMATO_WINDOW_DERIVE) then
        return YamatoFire("W_AttackDualDash")
    end
    YAMATO_FRAME.combo = combo
    return FALSE
end

-- AttackCommonFunction 开头调用。动作后段复位的那次调用连段数是 0，不换招
function YamatoCombo(r1, r2, l1, b1, b2, combo_count)
    local combo = YAMATO_FRAME.combo
    YAMATO_FRAME.combo = nil
    if combo == nil or (combo_count == 0 and combo.always ~= TRUE) then
        return r1, r2, l1, b1, b2
    end
    local next_b1 = combo.b1 or b1
    if combo.step ~= nil and YamatoHas(YAMATO_WINDOW_STEP) then
        next_b1 = combo.step
    elseif combo.devil ~= nil and YamatoHas(YAMATO_DEVIL) then
        next_b1 = combo.devil
    elseif combo.back ~= nil and YamatoBackInput() then
        next_b1 = combo.back
    end
    return combo.r1 or r1, combo.r2 or r2, combo.l1 or l1, next_b1, combo.b2 or b2
end

-- 双手轻攻击入口：后拉后撤斩，前推加 L2 突刺
function YamatoBothLight(b1, b2)
    if not YamatoEquipped() then
        return b1
    end
    if b1 == YAMATO_BOTH1 and b2 == "W_AttackBothHeavy1Start" then
        if YamatoBackInput() then
            return "W_AttackBothBackstep"
        end
        if math.abs(GetVariable("MoveAngle")) < YAMATO_FRONT_ANGLE and GetVariable("MoveSpeedLevel") > 0 and env(ActionDuration, ACTION_ARM_L2) > 0 then
            return "W_AttackBothDash"
        end
    elseif (b1 == "W_AttackBothDash" and b2 == "W_AttackBothHeavyDash") or (b1 == "W_AttackBothLightStep" and b2 == "W_AttackBothHeavy1Start") then
        return YAMATO_BOTH1
    end
    return b1
end

-- 双手重攻击入口都换成连斩起手
function YamatoBothHeavy(b1, b2)
    if not YamatoEquipped() then
        return b2
    end
    if (b1 == YAMATO_BOTH1 and b2 == "W_AttackBothHeavy1Start") or (b1 == "W_AttackBothDash" and b2 == "W_AttackBothHeavyDash") then
        return YAMATO_DUAL1
    end
    return b2
end

-- L2：有派生窗口放次元斩，跑动中连斩，后拉后撤；都不满足就什么也不做。不是阎魔刀返回 nil
function YamatoL2Request()
    if not YamatoEquipped() then
        return nil
    end
    if YamatoHas(YAMATO_WINDOW_DERIVE) then
        YamatoFire("W_AttackBothHeavySpecial1Start")
        return ATTACK_REQUEST_YAMATO
    elseif GetVariable("MoveSpeedLevel") > 1 then
        YamatoFire("W_AttackDualLight4")
        return ATTACK_REQUEST_YAMATO
    elseif YamatoBackInput() then
        YamatoFire("W_AttackBothHeavyDash")
        return ATTACK_REQUEST_YAMATO
    end
    return ATTACK_REQUEST_INVALID
end

-- L1 不出左手攻击（幻影剑在 YamatoFrame）；解锁且按住 L2、不在冷却时放次元斩绝
function YamatoL1Request()
    if YamatoHas(VERSATILITY_UNLOCK) and not YamatoHas(YAMATO_FINISHER_COOLDOWN) and env(ActionDuration, ACTION_ARM_L2) > 0 then
        act(AddSpEffect, YAMATO_FINISHER_COOLDOWN)
        YamatoFire(YAMATO_FINISHER)
        return ATTACK_REQUEST_YAMATO
    end
    return ATTACK_REQUEST_INVALID
end

-- 按方向选阵式；站着不动且没有左右后输入时返回 nil，退回普通发射
function YamatoMirageFormation()
    local angle = GetVariable("MoveAngle")
    if math.abs(angle) <= YAMATO_FRONT_ANGLE and GetVariable("MoveSpeedLevel") > 0 then
        return YAMATO_MIRAGE_FRONT
    elseif math.abs(angle + YAMATO_SIDE_ANGLE) <= YAMATO_FRONT_ANGLE then
        return YAMATO_MIRAGE_LEFT
    elseif math.abs(angle - YAMATO_SIDE_ANGLE) <= YAMATO_FRONT_ANGLE then
        return YAMATO_MIRAGE_RIGHT
    elseif math.abs(angle) > YAMATO_REAR_ANGLE then
        return YAMATO_MIRAGE_BACK
    end
    return nil
end

-- 幻影剑（Update 每帧调用，和动作状态无关）：按住 L1 连发，左右交替；同时按住跑步键放方向阵式
function YamatoMirage()
    if not YamatoEquipped() or env(ActionDuration, ACTION_ARM_L1) <= 0 or YamatoHas(YAMATO_MIRAGE_COOLDOWN) then
        return
    end
    act(AddSpEffect, YAMATO_MIRAGE_COOLDOWN)
    local formation = nil
    if env(ActionDuration, ACTION_ARM_L3) > 0 then
        formation = YamatoMirageFormation()
    end
    if formation ~= nil then
        act(AddSpEffect, formation)
    elseif YamatoHas(YAMATO_MIRAGE_SIDE_A) then
        act(AddSpEffect, YAMATO_MIRAGE_SIDE_B)
        act(AddSpEffect, YAMATO_MIRAGE_SHOT_A)
    else
        act(AddSpEffect, YAMATO_MIRAGE_SIDE_A)
        act(AddSpEffect, YAMATO_MIRAGE_SHOT_B)
    end
end

-- 解除动作走完或换下阎魔刀时清掉魔人效果，变身外观随之消失
function YamatoDevilUpdate()
    if YamatoHas(YAMATO_DEVIL) and (YamatoHas(YAMATO_DEVIL_END) or not YamatoEquipped()) then
        act(ClearSpEffect, YAMATO_DEVIL)
    end
end

-- Update 每帧调用
function YamatoFrame()
    YamatoMirage()
    YamatoDevilUpdate()
end

-- 魔人化：玩家切换单双持时接变身 / 解除动作。包一层原函数，不改它的内容；只有原函数返回 FALSE 表示没切换
-- 玩家按键走 ExecWeaponChange，is_force 是 TRUE（FALSE 那条路原函数只放行 COM），不能按 is_force 过滤；左手武器的切换不算
YAMATO_BASE_HAND_CHANGE = ExecHandChange
function ExecHandChange(hand, is_force, blend_type, no_reset)
    local style = c_Style
    local result = YAMATO_BASE_HAND_CHANGE(hand, is_force, blend_type, no_reset)
    if result == FALSE or not YamatoEquipped() or style == HAND_LEFT_BOTH or (style == HAND_RIGHT and hand == HAND_LEFT) then
        return result
    end
    if YamatoHas(YAMATO_DEVIL) then
        ExecEventAllBody(YAMATO_DEVIL_LEAVE)
    else
        ExecEventAllBody(YAMATO_DEVIL_ENTER)
    end
    return TRUE
end

