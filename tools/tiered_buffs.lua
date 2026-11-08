-- ==== NRStatAlloc 三档锻体（tools\tiered_hks.ps1 插入；效果 ID 与 relics\src\tiered\scripted.rs 对应）====
-- 锻体效果只是标记，这里按标记在放绝招 / 放技艺 / 每帧时施加实际效果，或拦下技艺与绝招

-- 绝招：所有角色的绝招动画开头都挂「绝招中」6999615，出现的那一刻视为放了绝招（含免费绝招与变身类），消失时动画结束
TIERED_ULT_ACTIVE = FALSE
-- 金5：本次绝招结束后要补满量表
TIERED_REFILL_PENDING = FALSE
-- 金13：按下免费绝招时的量表（-1 为没有），非送葬者的绝招动画照样扣量表，结束后还回去
TIERED_FREE_ULT_GAUGE = -1

-- 多效果锻体（relics\src\tiered\kept.rs）：{ 标记, 第一个子效果, 子效果数 }，标记在身上时把子效果保持挂着（每 15 帧补一次）
TIERED_KEEP = {
    { 9971270, 9971271, 2 },  -- 金7：周围异常时提升攻击力
    { 9971170, 9971500, 18 }, -- 紫17：每面盾提升精力上限
    { 9971180, 9971520, 12 }, -- 紫18：每把法杖 / 圣印记提升专注值上限
    { 9971190, 9971540, 6 },  -- 紫19：每面大盾提升血量上限
    { 9971390, 9971391, 2 },  -- 执行者1：周围异常时叠不屈
    { 9971400, 9971401, 2 },  -- 女爵7：周围异常时隐身
    { 9971410, 9971411, 2 },  -- 铁之眼6：周围异常时提升观察力
}
TIERED_KEEP_FRAME = 0

-- 按 ID 记上一帧有没有，用于判断效果刚出现 / 刚消失
TIERED_SEEN = {}
-- 复仇者5：等不死挂上才给本次绝招借出的复活次数 9971493（效果行说明见 relics\src\tiered\aura.rs）
TIERED_REVIVE_PENDING = false

function TieredHas(id)
    return env(GetSpEffectID, id) == TRUE
end

-- 倒地时是否走自行复活的起身动作（替换原版的 8970061 判断）：原版安定者复活由起身动画消耗，复仇者5 借出的次数在这里用掉
function TieredIsAutoRevival()
    if env(GetSpEffectID, 8970061) == TRUE then
        return TRUE
    end
    if TieredHas(9971493) then
        act(ClearSpEffect, 9971493)
        return TRUE
    end
    return FALSE
end

-- 刚出现返回 1，刚消失返回 -1，否则 0；每个 ID 每帧只能调用一次
function TieredEdge(id)
    local now = TieredHas(id)
    local before = TIERED_SEEN[id] == true
    TIERED_SEEN[id] = now
    if now and not before then
        return 1
    elseif before and not now then
        return -1
    end
    return 0
end

function TieredKeepApplied()
    for i = 1, #TIERED_KEEP, 1 do
        local group = TIERED_KEEP[i]
        if TieredHas(group[1]) then
            for id = group[2], group[2] + group[3] - 1, 1 do
                if not TieredHas(id) then
                    act(AddSpEffect, id)
                end
            end
        end
    end
end

-- 动作事件出现时挂效果 { 标记, 要挂的效果, 动作事件名前缀 }：新武器词条（relics\src\tiered\passives.rs）持有时战技 / 法术霸体，
-- 无赖1 开始架势时补「强化已就绪」、无赖5 重攻击霸体减伤（role_actions.rs）。
-- 原 ExecEvent 改名为 TieredExecEventBase；按事件名前缀判断，没有字符串库时不处理，不影响原动作
TIERED_EVENT_EFFECTS = {
    { 9972000, 9972001, { "W_SwordArts", "W_DemonSwordArts", "W_DrawStance" } },
    { 9972010, 9972011, { "W_MagicLaunch", "W_MagicFire", "W_MagicLoop" } },
    { 9971750, 704011, { "W_SmashAttackStanceStart" } },
    { 9971755, 9971756, { "W_AttackRightHeavy", "W_AttackBothHeavy" } },
}

function TieredOnEvent(state)
    if string == nil or state == nil then
        return
    end
    for i = 1, #TIERED_EVENT_EFFECTS, 1 do
        local group = TIERED_EVENT_EFFECTS[i]
        local prefixes = group[3]
        for j = 1, #prefixes, 1 do
            if string.sub(state, 1, string.len(prefixes[j])) == prefixes[j] then
                if TieredHas(group[1]) then
                    act(AddSpEffect, group[2])
                end
                return
            end
        end
    end
end

-- 执行者8：拔出妖刀时招式换成熔炉系列（动作在 tools\tiered_moveset.ps1）
function TieredCrucibleDrawn()
    if TieredHas(9971865) and IsDemonSwordMode() == TRUE then
        return TRUE
    end
    return FALSE
end

-- 脚本里的战技编号也当作月隐（178，没有特殊分支），手上的刀是别的架势战技时才不会走错派生
function TieredStanceArtId(art_id)
    if TieredCrucibleDrawn() == TRUE then
        return 178
    end
    return art_id
end

-- 技艺进纳刀架势（同月隐，架势动作由 DLL 的 crucible_sword 钩子选到月隐的片段），走法同 ExecArtsStance
function TieredEnterStance(blend_type)
    if TieredHas(9971865) == false then
        return FALSE
    end
    if c_IsStealth == TRUE or blend_type == nil then
        blend_type = ALLBODY
    end
    if blend_type == ALLBODY and MoveStart(LOWER, Event_Move, FALSE) == TRUE then
        blend_type = UPPER
    end
    ExecEventHalfBlend(Event_DrawStanceRightStart, blend_type)
    SetVariable("SwordArtsOneShotComboCategory", 0)
    SetSwordArtsPointInfo(ACTION_ARM_L2, TRUE)
    SetAIActionState()
    return TRUE
end

-- 第一下重攻击：a293 没有松手动作，走第二下那组（奥陶漩涡）
-- 架势轻攻击：原来的解放斩 / 未解放斩；架势重攻击：已解放是志留亚漩涡（不消耗解放），未解放是百相之羽
function TieredMovesetEvent(state)
    if state ~= "W_AttackBothHeavy1Start" and state ~= "W_AttackBothHeavy1SubStart" and state ~= "W_DrawStanceRightAttackLight" and state ~= "W_DrawStanceRightAttackHeavy" then
        return state
    end
    if TieredCrucibleDrawn() == FALSE then
        return state
    end
    local unleashed = env(GetSpEffectID, 707051) == TRUE
    if state == "W_DrawStanceRightAttackLight" then
        SetVariable("IndexDemonSwordArts", unleashed and 0 or 1)
        return "W_DemonSwordArts"
    elseif state == "W_DrawStanceRightAttackHeavy" then
        return unleashed and "W_AttackRightHeavy1Start" or "W_AttackRightHeavy2Start"
    end
    return "W_AttackBothHeavy2Start"
end

function ExecEvent(state)
    state = TieredCrucibleEvent(TieredMovesetEvent(state))
    TieredOnEvent(state)
    TieredExecEventBase(state)
end

-- 紫2：满血时翻滚 / 后跳的精力消耗打七五折；原函数改名为 TieredAddStaminaBase
function AddStamina(num)
    if (num == STAMINA_REDUCE_ROLLING or num == STAMINA_REDUCE_BACKSTEP) and TieredHas(9971160) and env(GetHP) >= env(GetMaxHP) then
        num = num * 0.75
    end
    return TieredAddStaminaBase(num)
end

-- 隐士1：不再吸收属性痕，冷却中（DLL 没填满属性槽）按技艺没有反应
function TieredCocktailBlocked()
    return env(GetHeroID) == HERO_MAGIC and TieredHas(9971860) and env(GetMagicCocktailAffinityCount) ~= 3
end

-- 金10 不能放绝招，金11 不能放技艺，执行者8 熔炉形态期间两样都不能放；原函数改名为 TieredGetAttackRequestBase
function GetAttackRequest(is_guard, blend_type)
    if TieredCrucibleCheckEnd() == TRUE then
        return ATTACK_REQUEST_INVALID
    end
    local request = TieredGetAttackRequestBase(is_guard, blend_type)
    if request == ATTACK_REQUEST_ULTRA and TieredHas(9971230) then
        return ATTACK_REQUEST_INVALID
    elseif request == ATTACK_REQUEST_SKILL and (TieredHas(9971240) or TieredCocktailBlocked()) then
        return ATTACK_REQUEST_INVALID
    elseif TieredCrucibleBlocks(request) then
        return ATTACK_REQUEST_INVALID
    end
    if request == ATTACK_REQUEST_ULTRA and env(GetHeroID) ~= HERO_ATTACKER and TieredHas(709300) then
        TIERED_FREE_ULT_GAUGE = TieredGaugePercent()
    end
    return request
end

-- 绝招量表 0~100（脚本里看不出单位，0~1 的小数也按百分比换算）
function TieredGaugePercent()
    local p = env(GetUltimateChargePercent)
    if p > 0 and p < 1 then
        p = p * 100
    end
    return p
end

-- 按 64 / 32 / … / 1 拆开，施加对应的加量表效果 9971257 … 9971251
function TieredRefundGauge(amount)
    local bit = 64
    local id = 9971257
    while bit >= 1 do
        if amount >= bit then
            act(AddSpEffect, id)
            amount = amount - bit
        end
        bit = bit / 2
        id = id - 1
    end
end

-- 女爵、送葬者的技艺不走 GetAttackRequest，在各自的开始处检查
function TieredSkillBlocked()
    return TieredHas(9971240)
end

function TieredOnSkill()
    local hero = env(GetHeroID)
    -- 执行者再按一次是收刀、追踪者钩爪途中再按是飞身，都不算放技艺
    if hero == HERO_TECHNICAL and GetVariable("AddDemonSwordModeBlend") ~= 0 then
        return
    elseif hero == HERO_BALANCE and TieredHas(104900) then
        return
    end
    -- 金10：放技艺后 10 秒攻击力提升、命中回血
    if TieredHas(9971230) then
        act(AddSpEffect, 9971231)
    end
    -- 守护者4：自己与附近队友（子弹 900997101）获得振奋香
    if TieredHas(9971450) then
        act(AddSpEffect, 9971451)
        act(AddSpEffect, 503500)
        act(AddSpEffect, 503501)
    end
    TieredOnSkillRoles(hero)
end

-- 第四批：放绝招时给附近队友发子弹（9971xx1 → 900997xxx），自己那份直接施加
function TieredOnUltimateAura()
    -- 女爵5：标记，隐身结束时再隐身
    if TieredHas(9971420) then
        act(AddSpEffect, 9971421)
        act(AddSpEffect, 9971422)
    end
    -- 女爵6：隐身时魔力攻击力提升
    if TieredHas(9971430) then
        act(AddSpEffect, 9971431)
        act(AddSpEffect, 9971432)
    end
    -- 无赖2 / 3：30 秒减伤与攻击力、霸体
    if TieredHas(9971460) then
        act(AddSpEffect, 9971461)
        act(AddSpEffect, 9971462)
    end
    if TieredHas(9971470) then
        act(AddSpEffect, 9971471)
        act(AddSpEffect, 9971472)
    end
    -- 复仇者4：标记，不死结束时再延长
    if TieredHas(9971480) then
        act(AddSpEffect, 9971481)
        act(AddSpEffect, 9971482)
    end
    -- 复仇者5：1 分钟的复活计时
    if TieredHas(9971490) then
        act(AddSpEffect, 9971491)
        act(AddSpEffect, 9971492)
    end
end

-- 第四批：原版效果消失时补上延长（每个人自己的脚本处理自己身上的标记）
function TieredUpdateAura()
    local finale = TieredEdge(703500)
    local march_ally = TieredEdge(705000)
    local march_self = TieredEdge(705010)
    local revive_timer = TieredEdge(9971492)
    -- 女爵5：绝招隐身结束后再隐身 7.5 秒
    if finale < 0 and TieredHas(9971422) then
        act(AddSpEffect, 9971423)
    end
    -- 复仇者4：不死结束后再延长 7.5 秒
    if TieredHas(9971482) then
        if march_ally < 0 then
            act(AddSpEffect, 9971483)
        end
        if march_self < 0 then
            act(AddSpEffect, 9971484)
        end
    end
    -- 复仇者5：计时开始后等身上挂了绝招的不死才给复活次数（本来就有安定者复活的不借），次数自带时长，到时自己消失
    if revive_timer > 0 then
        TIERED_REVIVE_PENDING = true
    elseif revive_timer < 0 then
        TIERED_REVIVE_PENDING = false
    end
    if TIERED_REVIVE_PENDING and (TieredHas(705000) or TieredHas(705010)) then
        TIERED_REVIVE_PENDING = false
        if not TieredHas(8970061) then
            act(AddSpEffect, 9971493)
        end
    end
    -- 送葬者3：恍惚期间补挂霸体
    if TieredHas(9971496) and (TieredHas(709000) or TieredHas(709010)) and not TieredHas(9971497) then
        act(AddSpEffect, 9971497)
    end
end

function TieredOnUltimate()
    -- 金5：绝招在动画中段发动时才扣量表，记下来等动画结束再补满；补满 / 冷却期间不再触发
    if TieredHas(9971210) and not TieredHas(9971212) and not TieredHas(9971213) then
        TIERED_REFILL_PENDING = TRUE
    end
    -- 紫1：10 秒内击杀返还一次量表
    if TieredHas(9971130) then
        act(AddSpEffect, 9971131)
    end
    -- 紫14：回复血量
    if TieredHas(9971150) then
        act(AddSpEffect, 9971151)
    end
    -- 金13：送葬者以外用掉免费绝招（送葬者的免费绝招动画自带 709301）；不是免费绝招就丢掉按键时记的量表
    if env(GetHeroID) ~= HERO_ATTACKER and TieredHas(709300) then
        act(AddSpEffect, 709301)
    else
        TIERED_FREE_ULT_GAUGE = -1
    end
    TieredOnUltimateAura()
end

function TieredOnUltimateEnd()
    -- 金5：补满量表（9971212 结束后换成 30 秒冷却 9971213）
    if TIERED_REFILL_PENDING == TRUE then
        TIERED_REFILL_PENDING = FALSE
        act(AddSpEffect, 9971212)
    end
    -- 金13：还回免费绝招扣掉的量表
    if TIERED_FREE_ULT_GAUGE >= 0 then
        TieredRefundGauge(TIERED_FREE_ULT_GAUGE)
        TIERED_FREE_ULT_GAUGE = -1
    end
end

function TieredUpdate()
    TIERED_KEEP_FRAME = TIERED_KEEP_FRAME + 1
    if TIERED_KEEP_FRAME >= 15 then
        TIERED_KEEP_FRAME = 0
        TieredKeepApplied()
    end
    TieredUpdateAura()
    TieredUpdateRoles()
    TieredUpdateCrucible()
    if TieredHas(6999615) then
        if TIERED_ULT_ACTIVE == FALSE then
            TIERED_ULT_ACTIVE = TRUE
            TieredOnUltimate()
        end
    elseif TIERED_ULT_ACTIVE == TRUE then
        TIERED_ULT_ACTIVE = FALSE
        TieredOnUltimateEnd()
    end
    -- 金13：送葬者以外没有紫色量表，免费绝招（709300）在身上时挂状态栏图标，用掉或到期就清掉
    if env(GetHeroID) ~= HERO_ATTACKER and TieredHas(709300) then
        if not TieredHas(9971258) then
            act(AddSpEffect, 9971258)
        end
    elseif TieredHas(9971258) then
        act(ClearSpEffect, 9971258)
    end
    -- 紫6：圣杯瓶为 0 时挂上减伤（不限时），有瓶时移除
    if TieredHas(9971140) and env(GetFlaskCount) == 0 then
        if not TieredHas(9971141) then
            act(AddSpEffect, 9971141)
        end
    elseif TieredHas(9971141) then
        act(ClearSpEffect, 9971141)
    end
    -- 守护者12：强化防御中（701530）补挂不耗精力
    if TieredHas(9971370) and TieredHas(701530) and not TieredHas(9971371) then
        act(AddSpEffect, 9971371)
    end
end
-- ==== NRStatAlloc 三档锻体 end ====

