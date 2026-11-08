<#
  把 GoldenAge 武器的连段脚本（tools\ga_yamato.lua）接进 pkg 的 c0000.hks，跟在 tiered_hks.ps1 之后运行。
  法环的写法是在每个攻击状态里给局部变量赋值，黑夜君临这些状态把招式名直接写在 AttackCommonFunction 的参数里，不能照贴：
    攻击状态 onUpdate      开头调用 YamatoUpdate(状态名)，转段就返回，否则记下这一帧的下一招
    AttackCommonFunction   开头按记下的下一招换参数（所有攻击状态都走这里）
    ExecAttack             双手轻 / 重入口换招；多一个请求类型表示脚本已放出招式，只做收尾
    L2                     不按绝招键时先问 YamatoL2Request（不是阎魔刀返回 nil，走原逻辑）
    L1 / 防御              阎魔刀的 L1 放幻影剑（YamatoFrame）和次元斩绝（YamatoL1Request）：不出左手攻击，也不能防御，同 GA 的禁防名单
    Update                 每帧调 YamatoFrame（幻影剑、魔人解除）和 VersatilityFrame（多才多艺，不分武器）
    ExecHandChange         ga_yamato.lua 里包一层接魔人化，这里不用插行
  每处命中次数都要对得上，整合包更新后对不上会直接报错。重复运行也会报错
#>
param([Parameter(Mandatory)] [string]$Hks)
$ErrorActionPreference = "Stop"
$path = (Resolve-Path $Hks).Path
$utf8 = New-Object Text.UTF8Encoding($false)
$text = [IO.File]::ReadAllText($path, $utf8)
if ($text.Contains("YamatoUpdate")) { throw "$path already has the GoldenAge hooks" }

function Edit-Line([string]$Line, [string]$What, [scriptblock]$Insert) {
    $re = "(?m)^([ \t]*)" + [regex]::Escape($Line) + "(\r?)$"
    $n = ([regex]::Matches($script:text, $re)).Count
    if ($n -ne 1) { throw "c0000.hks: expected 1 '$Line' ($What), found $n" }
    $script:text = [regex]::Replace($script:text, $re, { param($m) & $Insert $m.Groups[1].Value $m.Groups[2].Value })
}

$States = "AttackRightLight2", "AttackRightLight3", "AttackRightLight4", "AttackRightLight5", "AttackRightLight6",
    "AttackRightHeavySpecial1Start", "AttackRightHeavySpecial1End", "AttackRightHeavyCounter", "AttackRightLightCounter", "AttackRightSpecial1",
    "AttackDualLight1", "AttackDualLight2", "AttackDualLight3", "AttackDualLight4", "AttackDualLight5", "AttackDualLight6",
    "AttackDualDash", "AttackDualRolling", "AttackDualBackStep",
    "AttackBothLight1", "AttackBothLight2", "AttackBothLight3", "AttackBothLight4", "AttackBothLight5", "AttackBothLight6",
    "AttackBothHeavy1Start", "AttackBothHeavy1End", "AttackBothHeavy2Start", "AttackBothHeavy2End",
    "AttackBothHeavySpecial1Start", "AttackBothHeavySpecial1End", "AttackBothHeavySpecial2Start", "AttackBothHeavySpecial2End",
    "AttackBothSpecial1", "AttackBothSpecial2", "AttackBothDash", "AttackBothHeavyDash", "AttackBothLightStep", "AttackBothBackstep", "AttackBothHeavyCounter"
foreach ($s in $States) {
    $name = $s
    Edit-Line "function ${s}_onUpdate()" "$s" { param($i, $cr) "${i}function ${name}_onUpdate()$cr`n${i}    if YamatoUpdate(`"$name`") == TRUE then return end$cr" }.GetNewClosure()
}
"c0000.hks: $($States.Count) 个攻击状态接入连段"

# 多才多艺：攻击状态每帧挂自己的标记（放在连段判断前，转段那一帧也算）
. (Join-Path $PSScriptRoot "ga_versatility.ps1")
$markers = 0
for ($k = 0; $k -lt $VersatilityStates.Count; $k++) {
    $name = $VersatilityStates[$k]
    if (-not $name) { continue }
    $marker = $VersatilityBase + $k
    Edit-Line "function ${name}_onUpdate()" "$name" { param($i, $cr) "${i}function ${name}_onUpdate()$cr`n${i}    act(AddSpEffect, $marker)$cr" }.GetNewClosure()
    $markers++
}
"c0000.hks: $markers 个攻击状态接入多才多艺"

$signature = "function AttackCommonFunction(r1, r2, l1, l2, b1, b2, guardcondition, use_atk_queue, comboCount, gen_hand)"
Edit-Line $signature "连段换招" { param($i, $cr) "$i$signature$cr`n$i    r1, r2, l1, b1, b2 = YamatoCombo(r1, r2, l1, b1, b2, comboCount)$cr" }
Edit-Line "elseif request == ATTACK_REQUEST_BOTH_LIGHT then" "双手轻攻击入口" { param($i, $cr) "${i}elseif request == ATTACK_REQUEST_BOTH_LIGHT then$cr`n$i    b1 = YamatoBothLight(b1, b2)$cr" }
Edit-Line "elseif request == ATTACK_REQUEST_BOTH_HEAVY then" "双手重攻击入口" { param($i, $cr) "${i}elseif request == ATTACK_REQUEST_BOTH_HEAVY then$cr`n$i    b2 = YamatoBothHeavy(b1, b2)$cr" }
Edit-Line "elseif request == ATTACK_REQUEST_LEFT_REVERSAL then" "脚本已放出招式的收尾" { param($i, $cr) "${i}elseif request == ATTACK_REQUEST_YAMATO then$cr`n${i}elseif request == ATTACK_REQUEST_LEFT_REVERSAL then$cr" }
Edit-Line 'act(DebugLogOutput, "action request ACTION_ARM_L2")' "L2 入口" {
    param($i, $cr) "${i}act(DebugLogOutput, `"action request ACTION_ARM_L2`")$cr`n${i}if env(ActionDuration, ACTION_ARM_ACTION) <= 0 then$cr`n$i    local yamato_request = YamatoL2Request()$cr`n$i    if yamato_request ~= nil then$cr`n$i        return yamato_request$cr`n$i    end$cr`n${i}end$cr"
}
Edit-Line "if request_l1 == TRUE then" "L1 入口" { param($i, $cr) "${i}if request_l1 == TRUE then$cr`n$i    if YamatoEquipped() then return YamatoL1Request() end$cr" }
Edit-Line "function IsEnableGuard()" "禁用防御" { param($i, $cr) "${i}function IsEnableGuard()$cr`n$i    if YamatoEquipped() then return FALSE end$cr" }
$lua = (("ga_yamato.lua", "ga_versatility.lua" | ForEach-Object { [IO.File]::ReadAllText((Join-Path $PSScriptRoot $_), $utf8) }) -join "") -replace "\r?\n", "`r`n"
Edit-Line "function Update()" "插入阎魔刀和多才多艺函数" { param($i, $cr) "$lua" + "function Update()$cr`n    YamatoFrame()$cr`n    VersatilityFrame()$cr" }
[IO.File]::WriteAllText($path, $text, $utf8)
"c0000.hks: 阎魔刀脚本接入完成"
