-- ==== NRStatAlloc 执行者8 绝招：熔炉形态（tools\tiered_hks.ps1 插入；效果编号见 tools\tiered_crucible.ps1）====
-- 带「熔炉妖刀」锻体的执行者放绝招不变野兽：扣 70 量表，挂「变身中」标记（DLL 见到它就把所有招式表都换成熔炉妖刀，不用先拔刀；
-- 只挂到踏地结束，否则待机等也被换成 a293 里没有的动作），直接播熔炉骑士踏地（tools\tiered_crucible_anim.ps1，a293 的 32050 槽位 =
-- 双手第 6 段轻攻击）：1.00 秒由动作事件挂熔炉形态（熔炉斧形套装、减伤增攻、不耗精力），1.07 / 1.10 秒骑士震屏与爆发，播完拔出妖刀。
-- 换装会重建角色模型，同一帧放的特效与震屏会丢，所以换装比它们早几帧。
-- 形态期间持续扣量（同原版野兽每秒约 1.8），量表扣完或再按绝招键结束：先放一次蓄力黄金之怒，几帧后解除。
-- 期间不能用技艺、疾跑 / L3 / 非战斗区都不收刀；变招：奥陶漩涡收招再按重击接熔炉百相之尾，未解放时架势轻攻击换熔炉百相之角
CRUCIBLE_SWORD_BUFF = 9971865
CRUCIBLE_FORM = 9971870
CRUCIBLE_CONSUME = 9971871
CRUCIBLE_DRAIN = 9971872
CRUCIBLE_POISE = 9971873
CRUCIBLE_AURA = 9971874
CRUCIBLE_AURA_STRONG = 9971875
CRUCIBLE_AURA_COOLDOWN = 9971878
CRUCIBLE_ACTIVE = 9971879
-- 蓄力黄金之怒的发出效果（踏地时由动作事件挂，结束时由脚本挂）
CRUCIBLE_WRATH = 9971880
-- 结束时放出黄金之怒后过这么多帧再换回原装（换装重建模型，同一帧放子弹会找不到挂点）
CRUCIBLE_END_DELAY_FRAMES = 6
-- 结束阶段：0 无，正数 = 放出黄金之怒后的倒数帧，CRUCIBLE_ENDING_CLEAR = 撤形态中
-- （形态可能一次撤不掉，每帧清到没有为止；一次绝招只放一次结束的冲击波，新绝招才复位）
CRUCIBLE_ENDING_CLEAR = -1
TIERED_CRUCIBLE_ENDING = 0
-- 执行者6：熔炉形态的队友回血从 2% 提到 5%
CRUCIBLE_ROAR_BUFF = 9971770
ULT_ACTIVE_MARK = 6999615
CRUCIBLE_STOMP_EVENT = "W_AttackBothLight6"
CRUCIBLE_COMBO_RESTART = "W_AttackBothLight1"
-- 变招（动作在 tools\tiered_crucible_arts.ps1）：借的状态与 a293 槽位
CRUCIBLE_TAIL_CHARGED_EVENT = "W_AttackBothSpecial1"
CRUCIBLE_TAIL_EVENT = "W_AttackBothHeavy1SubStart"
CRUCIBLE_HORNS_EVENT = "W_AttackBothSpecial2"
-- 奥陶漩涡：蓄力版收招时还在重攻击 2 起手，快版在重攻击 2 收尾
CRUCIBLE_VORTEX_CHARGED_NODE = "AttackBothHeavy2Start_CMSG"
CRUCIBLE_VORTEX_NODE = "AttackBothHeavy2End_CMSG"
-- 变身进度：0 无，1 等标记生效，2 踏地中；踏地被打断时过这么多帧结束踏地阶段（形态没了就补、没拔刀就拔）
TIERED_CRUCIBLE_STAGE = 0
TIERED_CRUCIBLE_FRAMES = 0
CRUCIBLE_FALLBACK_FRAMES = 300

function TieredCrucibleForm()
    return env(GetHeroID) == HERO_TECHNICAL and TieredHas(CRUCIBLE_FORM)
end

function TieredCrucibleDraw(blend_type)
    SetVariable("AddDemonSwordModeBlend", 1)
    ExecEventHalfBlend(Event_DemonSwordStanceStart, blend_type)
end

-- 主动结束或量表扣完：放一次黄金之怒，TieredUpdateCrucible 倒数完再解除
function TieredCrucibleFinish()
    if TIERED_CRUCIBLE_ENDING ~= 0 then
        return
    end
    act(AddSpEffect, CRUCIBLE_WRATH)
    TIERED_CRUCIBLE_ENDING = CRUCIBLE_END_DELAY_FRAMES
end

-- 原版量表不满时不产生绝招请求，形态中直接读绝招键（专用键，或按住动作键再按 R2）
function TieredCrucibleUltPressed()
    return env(ActionRequest, ACTION_ARM_ULTRA) == TRUE or (env(ActionRequest, ACTION_ARM_R2) == TRUE and env(ActionDuration, ACTION_ARM_ACTION) > 0)
end

-- 形态中按绝招键主动结束；GetAttackRequest 包装（与原版读按键同一时机）与每帧都会问，踏地没播完不收
function TieredCrucibleCheckEnd()
    if TIERED_CRUCIBLE_STAGE == 0 and TIERED_CRUCIBLE_ENDING == 0 and TieredCrucibleForm() and TieredCrucibleUltPressed() then
        TieredCrucibleFinish()
        return TRUE
    end
    return FALSE
end

function TieredCrucibleEnd()
    act(ClearSpEffect, CRUCIBLE_FORM)
    act(ClearSpEffect, CRUCIBLE_DRAIN)
    act(ClearSpEffect, CRUCIBLE_POISE)
    act(ClearSpEffect, CRUCIBLE_ACTIVE)
end

-- 绝招入口（替换 W_DemonBeastStart）：没有锻体返回 FALSE，照原版变野兽
function TieredCrucibleUlt(blend_type)
    if env(GetHeroID) ~= HERO_TECHNICAL or not TieredHas(CRUCIBLE_SWORD_BUFF) then
        return FALSE
    end
    -- 标记要在发踏地之前挂上，DLL 选片段时才看得到
    TIERED_CRUCIBLE_ENDING = 0
    act(AddSpEffect, CRUCIBLE_ACTIVE)
    act(AddSpEffect, CRUCIBLE_CONSUME)
    act(AddSpEffect, CRUCIBLE_POISE)
    -- 「绝招中」照原版挂上；这里直接触发放绝招的锻体，标记消失时 TieredUpdate 照常走结束流程
    act(AddSpEffect, ULT_ACTIVE_MARK)
    if TIERED_ULT_ACTIVE == FALSE then
        TIERED_ULT_ACTIVE = TRUE
        TieredOnUltimate()
    end
    -- 拔着刀就瞬间收起：拔刀姿势是 Master Blend 里按这个变量加权的叠加层（原版收刀开头也是置 0），
    -- 权重 0 后层里动作的事件停了，拔刀标识 707060 随之消失，刀就不显示；装备的武器由踏地的动作事件藏起
    if GetVariable("AddDemonSwordModeBlend") ~= 0 then
        SetVariable("AddDemonSwordModeBlend", 0)
    end
    TIERED_CRUCIBLE_FRAMES = 0
    -- 标记当帧已在身上就直接踏地；没有就等下一帧，免得 DLL 还没切招式表选错片段
    if env(GetSpEffectID, CRUCIBLE_ACTIVE) == TRUE then
        TieredCrucibleStomp()
    else
        TIERED_CRUCIBLE_STAGE = 1
    end
    return TRUE
end

function TieredCrucibleStomp()
    TIERED_CRUCIBLE_STAGE = 2
    ExecEventAllBody(CRUCIBLE_STOMP_EVENT)
end

-- 踏地状态的每帧（AttackBothLight6_onUpdate 开头调用）：整段接管，不走原版的连段 / 取消；
-- 播完或到收招取消点（动作事件 4 / 11）就拔出妖刀
function TieredCrucibleStompUpdate()
    if TIERED_CRUCIBLE_STAGE ~= 2 then
        return FALSE
    end
    if env(IsAnimEnd, 0) == TRUE or env(IsGeneralAnimCancelPossible) == TRUE or env(IsMoveCancelPossible) == TRUE then
        TieredCrucibleStompDone()
    end
    return TRUE
end

-- 踏地结束：形态没挂上（踏地被打断）就补上，摘掉变身中标记（之后只有拔刀时招式表才换成熔炉妖刀），拔刀
function TieredCrucibleStompDone()
    TIERED_CRUCIBLE_STAGE = 0
    if not TieredHas(CRUCIBLE_FORM) then
        act(AddSpEffect, CRUCIBLE_FORM)
    end
    act(ClearSpEffect, CRUCIBLE_ACTIVE)
    if IsDemonSwordMode() == FALSE then
        TieredCrucibleDraw(ALLBODY)
    end
end

-- 熔炉形态拔刀时的变招（在 TieredMovesetEvent 改写之后）：奥陶漩涡收招再按重击接之尾（蓄力版接蓄力之尾），
-- 未解放时架势轻攻击（已改写成妖刀技艺、序号 1 = 未解放斩）换成蓄力之角；没有就返回 nil
function TieredCrucibleArt(state)
    if not TieredCrucibleForm() or TieredCrucibleDrawn() == FALSE then
        return nil
    end
    if state == "W_AttackBothHeavy2Start" then
        if IsNodeActive(CRUCIBLE_VORTEX_CHARGED_NODE) == TRUE then
            return CRUCIBLE_TAIL_CHARGED_EVENT
        elseif IsNodeActive(CRUCIBLE_VORTEX_NODE) == TRUE then
            return CRUCIBLE_TAIL_EVENT
        end
    elseif state == "W_DemonSwordArts" and GetVariable("IndexDemonSwordArts") == 1 then
        return CRUCIBLE_HORNS_EVENT
    end
    return nil
end

-- 踏地占了第 6 段轻攻击、变招占了双手特殊 1 / 2：熔炉妖刀拔刀时原版走到这些状态（连段数到 6、部分武器的特殊攻击）都改回第 1 段，
-- 只放行变身踏地与变招
function TieredCrucibleEvent(state)
    local art = TieredCrucibleArt(state)
    if art ~= nil then
        return art
    end
    if TieredCrucibleDrawn() == TRUE and ((state == CRUCIBLE_STOMP_EVENT and TIERED_CRUCIBLE_STAGE ~= 2)
        or state == CRUCIBLE_TAIL_CHARGED_EVENT or state == CRUCIBLE_HORNS_EVENT) then
        return CRUCIBLE_COMBO_RESTART
    end
    return state
end

-- 普通之尾借的双手重攻击副起手（AttackBothHeavy1SubStart_onUpdate 开头调用）：熔炉妖刀拔刀时原版走不到这个状态，
-- 所以是变招；不照原版播完转去重攻击起手，按通用攻击状态处理连段与取消，播完回待机
function TieredCrucibleArtUpdate()
    if TieredCrucibleDrawn() == FALSE then
        return FALSE
    end
    if AttackCommonFunction("W_AttackBothLight1", "W_AttackBothHeavy1Start", "W_AttackBothLeft1", "W_AttackLeftHeavy1", "W_AttackBothLight1", "W_AttackBothHeavy1Start", FALSE, TRUE, 0) == TRUE then
        return TRUE
    end
    if env(IsAnimEnd, 0) == TRUE then
        ExecEventAllBody("W_Idle")
    end
    return TRUE
end

-- 持刀疾跑：疾跑时移动类型不走拔刀的架势移动（没有疾跑动作，会变回走路）
function TieredCrucibleSprint()
    return TieredCrucibleForm() and GetVariable("MoveSpeedIndex") == 3
end

-- 拔刀时上半身不在 Move_Upper Selector，MMV 的疾跑自动取消会误判；下半身在移动或疾跑起步时不取消
function TieredCrucibleMoving()
    return TieredCrucibleForm() and (GetLocomotionState() == PLAYER_STATE_MOVE or IsNodeActive("SprintStart_Selector") == TRUE)
end

-- 变身、熔炉形态与结束过程中拦下技艺与再次放绝招
function TieredCrucibleBlocks(request)
    return (request == ATTACK_REQUEST_SKILL or request == ATTACK_REQUEST_ULTRA)
        and (TieredCrucibleForm() or TIERED_CRUCIBLE_STAGE > 0 or TIERED_CRUCIBLE_ENDING ~= 0)
end

function TieredUpdateCrucible()
    if TIERED_CRUCIBLE_STAGE > 0 then
        TIERED_CRUCIBLE_FRAMES = TIERED_CRUCIBLE_FRAMES + 1
        if TIERED_CRUCIBLE_FRAMES > CRUCIBLE_FALLBACK_FRAMES then
            -- 踏地被打断（状态没走到播完）：结束踏地阶段
            TieredCrucibleStompDone()
        elseif TIERED_CRUCIBLE_STAGE == 1 and TieredHas(CRUCIBLE_ACTIVE) then
            TieredCrucibleStomp()
        elseif not TieredHas(CRUCIBLE_POISE) then
            act(AddSpEffect, CRUCIBLE_POISE)
        end
    end
    if TIERED_CRUCIBLE_ENDING > 0 then
        TIERED_CRUCIBLE_ENDING = TIERED_CRUCIBLE_ENDING - 1
        if TIERED_CRUCIBLE_ENDING == 0 then
            TIERED_CRUCIBLE_ENDING = CRUCIBLE_ENDING_CLEAR
        end
        return
    end
    if TIERED_CRUCIBLE_ENDING == CRUCIBLE_ENDING_CLEAR then
        if TieredHas(CRUCIBLE_FORM) or TieredHas(CRUCIBLE_DRAIN) then
            TieredCrucibleEnd()
        else
            TIERED_CRUCIBLE_ENDING = 0
        end
        return
    end
    if not TieredCrucibleForm() then
        return
    end
    if TieredGaugePercent() <= 0 then
        TieredCrucibleFinish()
        return
    end
    if TieredCrucibleCheckEnd() == TRUE then
        return
    end
    if not TieredHas(CRUCIBLE_DRAIN) then
        act(AddSpEffect, CRUCIBLE_DRAIN)
    end
    -- 霸体单独一条（类别 1001 会被同类别效果顶掉），掉了就补
    if not TieredHas(CRUCIBLE_POISE) then
        act(AddSpEffect, CRUCIBLE_POISE)
    end
    -- 每秒发一次给附近友方回血的子弹（不打自己）
    if not TieredHas(CRUCIBLE_AURA_COOLDOWN) then
        act(AddSpEffect, CRUCIBLE_AURA_COOLDOWN)
        if TieredHas(CRUCIBLE_ROAR_BUFF) then
            act(AddSpEffect, CRUCIBLE_AURA_STRONG)
        else
            act(AddSpEffect, CRUCIBLE_AURA)
        end
    end
end
-- ==== NRStatAlloc 执行者8 熔炉形态 end ====
