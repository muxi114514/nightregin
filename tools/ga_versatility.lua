-- ==== 多才多艺（GoldenAge，tools\ga_hks.ps1 插入）====
-- 不分武器：每个攻击状态 onUpdate 挂自己的标记效果（持续 20 秒，ga_hks 按 ga_versatility.ps1 的状态表插入），
-- 这里每帧数身上的标记有几种，只留对应层数的效果（时限很长，状态栏图标要常驻，短时效果反复补挂会闪）；0 种挂 0 层，只显示不加成
VERSATILITY_MARKER_FIRST = 103200
VERSATILITY_MARKER_LAST = 103257
-- 下标 = 招式种类数，满 10 种封顶
VERSATILITY_LAYERS = { 103260, 103260, 103261, 103262, 103263, 103264, 103265, 103266, 103268, 103270 }
VERSATILITY_MAX_KINDS = 10
VERSATILITY_LAYER_ZERO = 103267
-- 满层标记：部分武器的特殊招式要它（阎魔刀是次元斩绝）
VERSATILITY_UNLOCK = 103269
-- 世界之眼的效果：常驻满层，不用凑种类
VERSATILITY_TALISMAN = 10004040

-- 下标 1、2 是同一层，从 2 开始正好每层走一次
function VersatilitySetLayer(target)
    for i = 2, VERSATILITY_MAX_KINDS do
        local id = VERSATILITY_LAYERS[i]
        if id ~= target and env(GetSpEffectID, id) == TRUE then
            act(ClearSpEffect, id)
        end
    end
    if target ~= VERSATILITY_LAYER_ZERO and env(GetSpEffectID, VERSATILITY_LAYER_ZERO) == TRUE then
        act(ClearSpEffect, VERSATILITY_LAYER_ZERO)
    end
    if env(GetSpEffectID, target) ~= TRUE then
        act(AddSpEffect, target)
    end
end

function VersatilityFrame()
    if env(GetSpEffectID, VERSATILITY_TALISMAN) == TRUE then
        VersatilitySetLayer(VERSATILITY_LAYERS[VERSATILITY_MAX_KINDS])
        act(AddSpEffect, VERSATILITY_UNLOCK)
        return
    end
    local kinds = 0
    for id = VERSATILITY_MARKER_FIRST, VERSATILITY_MARKER_LAST do
        if env(GetSpEffectID, id) == TRUE then
            kinds = kinds + 1
            if kinds >= VERSATILITY_MAX_KINDS then
                break
            end
        end
    end
    local target = VERSATILITY_LAYER_ZERO
    if kinds > 0 then
        target = VERSATILITY_LAYERS[kinds]
    end
    VersatilitySetLayer(target)
    if kinds >= VERSATILITY_MAX_KINDS then
        act(AddSpEffect, VERSATILITY_UNLOCK)
    end
end

