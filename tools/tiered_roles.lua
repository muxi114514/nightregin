-- ==== NRStatAlloc 三档锻体 · 第五批角色（效果 ID 与 relics\src\tiered\role_scripts.rs 对应）====
-- 送葬者1：0 无事，1 正在续的 3 秒（709300 是本脚本挂的），2 刚由本脚本清掉（下一帧的消失不再续）
TIERED_RESONANCE = 0
-- 隐士3：属性痕层数拆成 7 位（9971640 = 1 层 … 9971646 = 64 层）
TIERED_RESIDUE_FIRST = 9971640
TIERED_RESIDUE_MAX = 127

-- 放技艺时（TieredOnSkill 调用）
function TieredOnSkillRoles(hero)
    -- 女爵1：原版遗物（7500300）的技艺无敌再挂一份更长的
    if hero == HERO_SPEED and TieredHas(9971625) and TieredHas(7500300) then
        act(AddSpEffect, 9971626)
    end
end

-- 送葬者1：共鸣 709300 自然结束时重新挂上，计时 9971631（3 秒）到了清掉；放绝招用掉的不补
function TieredUpdateResonance()
    local resonance = TieredEdge(709300)
    local timer = TieredEdge(9971631)
    if TIERED_RESONANCE == 1 and timer == -1 then
        TIERED_RESONANCE = 0
        if TieredHas(709300) then
            act(ClearSpEffect, 709300)
            TIERED_RESONANCE = 2
        end
        return
    end
    if resonance ~= -1 then
        return
    end
    if TIERED_RESONANCE == 2 then
        TIERED_RESONANCE = 0
    elseif TIERED_RESONANCE == 1 then
        -- 续上的那份被用掉了
        TIERED_RESONANCE = 0
        act(ClearSpEffect, 9971631)
    elseif TieredHas(9971630) and not TieredHas(6999615) then
        act(AddSpEffect, 709300)
        act(AddSpEffect, 9971631)
        TIERED_RESONANCE = 1
    end
end

-- 隐士3：每吸收一次属性痕（706000~706003 出现）多一层，层数按位挂效果
function TieredUpdateResidues()
    -- 隐士1 放魔法时由 DLL 挂吸收效果、不走吸收动作，遗物「魔法之境」在这里判断（原版在吸收结束的动作里）
    if TieredHas(9971860) then
        ExecElementalAbsorption_TerraMagicus()
    end
    local gained = 0
    for id = 706000, 706003 do
        if TieredEdge(id) == 1 then
            gained = gained + 1
        end
    end
    if gained == 0 or not TieredHas(9971635) then
        return
    end
    local count = 0
    local bit = 1
    for i = 0, 6 do
        if TieredHas(TIERED_RESIDUE_FIRST + i) then
            count = count + bit
        end
        bit = bit * 2
    end
    count = count + gained
    if count > TIERED_RESIDUE_MAX then
        count = TIERED_RESIDUE_MAX
    end
    for i = 0, 6 do
        local id = TIERED_RESIDUE_FIRST + i
        local want = count % 2 == 1
        if want and not TieredHas(id) then
            act(AddSpEffect, id)
        elseif not want and TieredHas(id) then
            act(ClearSpEffect, id)
        end
        count = (count - count % 2) / 2
    end
end

-- ==== 第七批角色（改原版判定，效果 ID 与 relics\src\tiered\role_actions.rs 对应；守护者9 的翻滚在 tiered_hks.ps1 里直接改条件）====
-- 执行者3：给过累积后等 30 帧才允许下一次（一次拨开可能连着判定几下，多给会把已解放的累积器再次攒满清空）
TIERED_RELEASE_WAIT = 0

-- 妖刀完美弹反判定（替换原版三处 707000 检查）：执行者2 延长的判定 9971761 也算；成立时执行者3 直接给累积器 +50
function TieredDemonJustGuard()
    if env(GetSpEffectID, 707000) == FALSE and not TieredHas(9971761) then
        return FALSE
    end
    if TIERED_RELEASE_WAIT == 0 and TieredHas(9971765) and not TieredHas(707051) then
        act(AddSpEffect, 9971766)
        TIERED_RELEASE_WAIT = 30
    end
    return TRUE
end

function TieredUpdateActions()
    if TIERED_RELEASE_WAIT > 0 then
        TIERED_RELEASE_WAIT = TIERED_RELEASE_WAIT - 1
    end
    -- 执行者2：完美弹反判定结束时再挂 0.15 秒副本
    if TieredEdge(707000) == -1 and TieredHas(9971760) then
        act(AddSpEffect, 9971761)
    end
    -- 执行者6：遗物咆哮回血出现时追加一半
    if TieredEdge(7011701) == 1 and TieredHas(9971770) then
        act(AddSpEffect, 9971771)
    end
    -- 学者6：弹反屏障结束时再挂 2 秒副本
    if TieredEdge(708050) == -1 and TieredHas(9971775) then
        act(AddSpEffect, 9971776)
    end
end

-- 每帧（TieredUpdate 调用）
function TieredUpdateRoles()
    TieredUpdateResonance()
    TieredUpdateResidues()
    TieredUpdateActions()
end
-- ==== NRStatAlloc 三档锻体 · 第五 / 七批角色 end ====
