<#
  把法环 mod GoldenAge 的物品移植进黑夜君临的 pkg（只给 MMV 系，tools\build_mmv_pkg.ps1 -GaPort 时调用，也可单独跑）。
  两边参数表行大小不同，不能整行拷：以黑夜君临同类行为底，按字段名覆盖要带过来的字段；每个物品只带自己引用到的内容，不合并整表。
    护符   EquipParamAccessory → 词条 AttachEffectParam → 效果 SpEffectParam（黑夜君临护符的 spEffectId_1~3 填的是词条 ID，详情页三个槽显示的也是词条）
    武器   EquipParamWeapon + BehaviorParam_PC（变体段）+ 缺的 AtkParam_Pc + parts 模型；基础攻击按模板的「黑夜君临 / 法环」比例换算
    文本   item_dlc01 的名字 / 说明 / 简介（中英）；图标   从 GA 的 BC7 图集转成 DXT5，放进 01_common_h 图集的空白处（要 Python + Pillow）
    招式类武器（有独立招式表的）另见 ga_closure.ps1（参数闭包），接着要跑 ga_moveset.ps1（招式表与动画）、ga_behavior.ps1（行为图片段）、
    ga_chr_dummy.ps1（角色挂点）、ga_hks.ps1（连段脚本）
  就地改 <Pkg> 下的 regulation.bin、msg、menu，模型写到 <Pkg>\parts。重复运行会报错（行已存在）。
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$GaRoot = "D:\steam\steamapps\common\ELDEN RING\Game\GA",
    [string]$ErRegulation = "D:\steam\steamapps\common\ELDEN RING\Game\regulation.bin",
    [string]$Defs = "D:\relink\114514\smithbox\Assets\PARAM",
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

# ---- 要移植的物品 ----
# Template = 黑夜君临里同类的行。Info：替换 GA 的说明（最多显示三行，每行约 21 个汉字，自动换行也算一行）。
# EffectSet：覆盖 GA 效果里的数值。Slots：详情页三个槽的词条；Effect = 这条词条挂的效果（第一条默认挂 GA 效果，其余默认只显示），
# Value = 显示的数值，Percent = 是否带 %，Icon = 状态图标（原版 20291 血量、20293 精力恢复、20208 大山羊）
$Talismans = @(
    @{ Id = 20008; Template = 1000; EffectTemplate = 310000; AttachTemplate = 310000  # 八刻轮印
        Info = @{ zhocn = "大幅提升各项属性"; engus = "Greatly raises all attributes" }
        # 精力恢复 = 绿龟护符（+8）的两倍，异常抵抗全部 +100；强韧度改由第三个词条的大山羊效果负责，这里还原 GA 的削韧减免
        EffectSet = @{ staminaRecoverChangeSpeed = 16; changePoisonResistPoint = 100; changeDiseaseResistPoint = 100; changeBloodResistPoint = 100
            changeCurseResistPoint = 100; changeFreezeResistPoint = 100; changeSleepResistPoint = 100; changeMadnessResistPoint = 100; toughnessDamageCutRate = 1 }
        Slots = @(
            @{ zhocn = "提升血量、专注值、精力、攻击力、减伤率"; engus = "Raises HP, FP, stamina, attack, negation"; Value = 20; Percent = $true; Icon = 20291 },
            @{ zhocn = "提升精力恢复速度、异常状态抵抗力"; engus = "Raises stamina recovery, status resistances"; Value = 100; Percent = $false; Icon = 20293 },
            @{ zhocn = "大幅提升强韧度"; engus = "Greatly raises poise"; Value = 9178; Percent = $false; Icon = 20208; Effect = 312101 }  # 大山羊护符的霸体
        )
    },
    # 世界之眼：多才多艺常驻 9 层（ga_versatility.lua 看效果判断）。效果本身不加攻击，词条的 18% 就是第 9 层的加成，只显示
    @{ Id = 20015; Template = 1000; EffectTemplate = 310000; AttachTemplate = 310000
        Info = @{ zhocn = "能够打开与关闭连接平行世界的门，与平行世界中的自己进行同调"; engus = "Opens and closes the doors to parallel worlds, attuning the wearer with their other selves" }
        EffectSet = @{ physicsAttackRate = 1; magicAttackRate = 1; fireAttackRate = 1; thunderAttackRate = 1; darkAttackRate = 1 }
        Slots = @(
            @{ zhocn = "提升18%攻击力"; engus = "Raises attack by 18%"; Value = 18; Percent = $true; Icon = 20533 },
            @{ zhocn = '"多才多艺"常驻为9层'; engus = '"Versatility" stays at layer 9'; Value = 9; Percent = $false; Icon = 20544 }
        )
    }
)
$Weapons = @(
    @{ Id = 14530000; Template = 14000000 }  # 王室双头斧：动作 30、战技 650 都已有
    # 阎魔刀：招式走普通攻击状态 + 特殊类别 73 的脚本（tools\ga_hks.ps1），招式表和动画见 tools\ga_moveset.ps1。
    # 模板只取强化和属性补正；Effects = 脚本直接挂或判断的效果（幻影剑、魔人、冷却、时机标记）；Transforms = 魔人化的变身防具；
    # Attach = 词条表，默认沿用模板武器（白色档），这里用传说太刀的三档表
    @{ Id = 89000000; Template = 9000000; Attach = 501000200; Moveset = $true; Taes = @("a73", "a373"); Transforms = @(99970000)
        Effects = @(10020070, 10020071, 10020072, 10020073, 10020074, 10020081, 10020082, 10020083, 10020084, 10020085, 10020087, 10020090, 10020091, 10020092, 10020093, 10020095, 103269) }
)

# 武器从 GA 带过来的字段，其余（强化、属性补正、需求、防御、绝招量表等）沿用黑夜君临模板
$WeaponFields = @("behaviorVariationId", "sortId", "weight", "equipModelId", "swordArtsParamId", "spAtkcategory", "defSeMaterial1", "defSfxMaterial1",
    "atkAttribute", "atkAttribute2", "isNormalAttackType", "isBlowAttackType", "isSlashAttackType", "isThrustAttackType", "saWeaponDamage")
$WeaponAttack = @("attackBasePhysics", "attackBaseMagic", "attackBaseFire", "attackBaseThunder", "attackBaseDark")
# 招式类武器多带：动作与常驻效果。模板的角色专属招式表要清掉（打刀行是执行者的 a277）
$MovesetFields = @("wepmotionCategory", "wepmotionOneHandId", "wepmotionBothHandId", "leftHandEquipable", "spAttribute", "staminaConsumptionRate",
    "residentSpEffectId", "residentSpEffectId1", "residentSpEffectId2", "spEffectBehaviorId0", "spEffectMsgId0", "absorpParamId") +
    (0..7 | ForEach-Object { "traceSfxId$_", "traceDmyIdHead$_", "traceDmyIdTail$_" })
$NoHeroMoveset = @{ heroMovesetOverrideID = 0; heroID = 0; heroBehaviorVariationID = -1 }
$ArtText = @(@{ From = 42, 331; To = 42 }, @{ From = 43, 332; To = 43 })
$OriginFields = @("originEquipWep") + (1..15 | ForEach-Object { "originEquipWep$_" })
$TalismanFields = @("weight", "sortId", "accessoryGroup", "rarity")
# 文本：(GA 里依次找的 FMG, 写进黑夜君临 item_dlc01 的 FMG)
$TalismanText = @(@{ From = 13, 316; To = 13 }, @{ From = 23, 317; To = 23 }, @{ From = 27, 318; To = 27 })
$WeaponText = @(@{ From = 11, 310; To = 11 }, @{ From = 21, 311; To = 21 }, @{ From = 25, 312; To = 25 })
$FmgAccessoryInfo = 23
$FmgAttachName = 44
$FmgAttachInfo = 50
$AttachSlots = "spEffectId_1", "spEffectId_2", "spEffectId_3"
# 新词条编号（同号作文本 ID）
$AttachBase = 9980000
$Langs = "zhocn", "engus"
$IconAtlas = "SB_ItemIcon_2"
# 黑夜君临的图标字段是 s16（GA 用 45000 以上放不下），新图标从原版 21886 之后的空闲段编号
$IconBase = 24000
$IconPx = 160
$IconStep = 164
$AtlasPx = 4096
$BlockBytes = 16
# 黑夜君临去掉了法环「对敌 / 对玩家伤害修正」的攻击字段（位置改作别的用途），对敌的改用攻击倍率，相乘写入；对玩家的 PvP 修正没有对应，丢弃
$EffectAlias = @{
    atkEnemyDmgCorrectRate_Physics = "physicsAttackRate"; atkEnemyDmgCorrectRate_Magic = "magicAttackRate"; atkEnemyDmgCorrectRate_Fire = "fireAttackRate"
    atkEnemyDmgCorrectRate_Thunder = "thunderAttackRate"; atkEnemyDmgCorrectRate_Dark = "darkAttackRate"
}
$CopyAll = "."
$EffectAlways = "^effectTarget"
$BehaviorBase = 100000000
$VariationStep = 1000
# 法环 BehaviorParam 的 refType：0 攻击、1 子弹、2 效果
$RefAttack = 0
$NamePrefix = "[GA] "
$ModeSample = 1500

$ErDef = @{ EquipParamWeapon = "EquipParamWeapon"; EquipParamAccessory = "EquipParamAccessory"; SpEffectParam = "SpEffect"; BehaviorParam_PC = "BehaviorParam"; AtkParam_Pc = "AtkParam"
    Bullet = "BulletParam"; SpEffectVfxParam = "SpEffectVfx"; SwordArtsParam = "SwordArtsParam"; WepAbsorpPosParam = "WepAbsorpPosParam"; EquipParamProtector = "EquipParamProtector" }
$NrDef =@{ EquipParamWeapon = "EquipParamWeapon"; EquipParamAccessory = "EquipParamAccessory"; SpEffectParam = "SpEffectParam"; BehaviorParam_PC = "BehaviorParam"; AtkParam_Pc = "AtkParam"
    Bullet = "Bullet"; SpEffectVfxParam = "SpEffectVfxParam"; SwordArtsParam = "SwordArtsParam"; WepAbsorpPosParam = "WepAbsorpPosParam"; EquipParamProtector = "EquipParamProtector" }

. (Join-Path $PSScriptRoot "ga_params.ps1")

$pkg = (Resolve-Path $Pkg).Path
$regPath = "$pkg\regulation.bin"
Invoke-InDeps {
    $script:nrBnd = [SoulsFormats.SFUtil]::DecryptNightreignRegulation($regPath)
    $script:gaBnd = [SoulsFormats.SFUtil]::DecryptERRegulation("$GaRoot\regulation.bin")
    $script:erBnd = [SoulsFormats.SFUtil]::DecryptERRegulation($ErRegulation)
}
$nr = @{}; $ga = @{}; $er = @{}
foreach ($t in $ErDef.Keys) {
    $nr[$t] = Open-Param $nrBnd $t "$Defs\NR\Defs\$($NrDef[$t]).xml"
    $ga[$t] = Open-Param $gaBnd $t "$Defs\ER\Defs\$($ErDef[$t]).xml"
}
foreach ($t in "EquipParamWeapon", "AtkParam_Pc", "Bullet", "SpEffectParam", "SpEffectVfxParam") { $er[$t] = Open-Param $erBnd $t "$Defs\ER\Defs\$($ErDef[$t]).xml" }
. (Join-Path $PSScriptRoot "ga_closure.ps1")
. (Join-Path $PSScriptRoot "ga_status.ps1")
. (Join-Path $PSScriptRoot "ga_versatility.ps1")
$nr.AttachEffectParam = Open-Param $nrBnd "AttachEffectParam" "$Defs\NR\Defs\AttachEffectParam.xml"
$nr.EquipParamCustomWeapon = Open-Param $nrBnd "EquipParamCustomWeapon" "$Defs\NR\Defs\EquipParamCustomWeapon.xml"

$icons = @()
$textPlan = @()
# 不取自 GA 的文本：@{ To = FMG; Id; zhocn; engus }
$ownText = @()
$nextAttach = $AttachBase
# GA 图标号 → 新图标号（同一个图标只搬一次）
function Use-Icon([int]$Source) {
    $hit = $script:icons | Where-Object { $_.Source -eq $Source } | Select-Object -First 1
    if ($hit) { return $hit.New }
    $new = $IconBase + $script:icons.Count
    $script:icons += @{ Source = $Source; New = $new }
    $new
}

# ---- 护符 ----
foreach ($item in $Talismans) {
    $src = Get-Row $ga.EquipParamAccessory $item.Id
    if (-not $src) { throw "GA talisman $($item.Id) not found" }
    $effectId = [int]$src["refId"].Value
    $srcEffect = Get-Row $ga.SpEffectParam $effectId
    if (-not $srcEffect) { throw "GA effect $effectId not found" }
    $effect = Add-Row $nr.SpEffectParam (Get-Row $nr.SpEffectParam $item.EffectTemplate) $effectId "Talisman $($item.Id) effect"
    $delta = Copy-Delta $srcEffect $effect $ga.SpEffectParam $nr.SpEffectParam $EffectAlias
    Clear-EffectUi $effect
    if ($item.EffectSet) { foreach ($field in $item.EffectSet.Keys) { Set-Cell $effect $field $item.EffectSet[$field] } }
    if ($delta.Dropped.Count) { "  效果 $effectId 丢弃（黑夜君临没有）: $($delta.Dropped -join ', ')" }
    $row = Add-Row $nr.EquipParamAccessory (Get-Row $nr.EquipParamAccessory $item.Template) $item.Id "Talisman"
    [void](Copy-Fields $src $row -Only $TalismanFields)
    foreach ($field in $AttachSlots) { Set-Cell $row $field -1 }
    $attachTemplate = Get-Row $nr.AttachEffectParam $item.AttachTemplate
    if ($item.Slots.Count -gt $AttachSlots.Count) { throw "talisman $($item.Id): too many slots" }
    for ($i = 0; $i -lt $item.Slots.Count; $i++) {
        $slot = $item.Slots[$i]
        $attachId = $nextAttach
        $nextAttach++
        $attach = Add-Row $nr.AttachEffectParam $attachTemplate $attachId "Talisman $($item.Id) slot $($i + 1)"
        $passive = if ($slot.Effect) { $slot.Effect } elseif ($i -eq 0) { $effectId } else { -1 }
        if ($passive -gt 0 -and -not (Get-Row $nr.SpEffectParam $passive)) { throw "talisman $($item.Id) slot $($i + 1): effect $passive not found" }
        Set-Cell $attach "passiveSpEffectId_1" $passive
        Set-Cell $attach "attachTextId" $attachId
        Set-Cell $attach "displayedModifierValue" $slot.Value
        Set-Cell $attach "displayPercentageSymbol" $(if ($slot.Percent) { 1 } else { 0 })
        Set-Cell $attach "statusIconId" $slot.Icon
        Set-Cell $row $AttachSlots[$i] $attachId
        foreach ($fmg in $FmgAttachName, $FmgAttachInfo) { $ownText += @{ To = $fmg; Id = $attachId; zhocn = $slot.zhocn; engus = $slot.engus } }
    }
    $icon = Use-Icon ([int]$src["iconId"].Value)
    Set-Cell $row "iconId" $icon
    $fmgs = @($TalismanText | Where-Object { -not ($item.Info -and $_.To -eq $FmgAccessoryInfo) })
    if ($item.Info) { $ownText += @{ To = $FmgAccessoryInfo; Id = $item.Id; zhocn = $item.Info.zhocn; engus = $item.Info.engus } }
    $textPlan += @{ Id = $item.Id; Fmgs = $fmgs }
    "护符 $($item.Id)：效果 $effectId（写入 $($delta.Copied) 个字段），图标 $($src['iconId'].Value) -> $icon"
}

# ---- 武器 ----
foreach ($item in $Weapons) {
    $src = Get-Row $ga.EquipParamWeapon $item.Id
    if (-not $src) { throw "GA weapon $($item.Id) not found" }
    $tpl = Get-Row $nr.EquipParamWeapon $item.Template
    $tplEr = Get-Row $er.EquipParamWeapon $item.Template
    if (-not $tpl -or -not $tplEr) { throw "weapon template $($item.Template) not found" }
    if (-not $item.Moveset -and [int]$src["wepmotionCategory"].Value -ne [int]$tpl["wepmotionCategory"].Value) { throw "weapon $($item.Id): template moveset differs" }
    $row = Add-Row $nr.EquipParamWeapon $tpl $item.Id "Weapon"
    [void](Copy-Fields $src $row -Only $WeaponFields)
    if ($item.Moveset) {
        [void](Copy-Fields $src $row -Only $MovesetFields)
        foreach ($f in $NoHeroMoveset.Keys) { Set-Cell $row $f $NoHeroMoveset[$f] }
    }
    foreach ($f in $OriginFields) { Set-Cell $row $f $item.Id }
    [void](Add-CustomWeapon $nr.EquipParamCustomWeapon $item.Id $item.Template $item.Attach)
    # 基础攻击按同模板两边的物理攻击比例换算，属性补正等沿用模板
    $scale = [double]$tpl["attackBasePhysics"].Value / [double]$tplEr["attackBasePhysics"].Value
    foreach ($f in $WeaponAttack) { Set-Cell $row $f ([Math]::Round([double]$src[$f].Value * $scale)) }
    # 变体段的判定行 + 引用到、黑夜君临还没有的攻击行
    $variation = [int]$src["behaviorVariationId"].Value
    $lo = $BehaviorBase + $variation * $VariationStep
    $behaviors = @($ga.BehaviorParam_PC.Param.Rows | Where-Object { $_.ID -ge $lo -and $_.ID -lt $lo + $VariationStep })
    $added = 0
    if ($item.Moveset) {
        Copy-MovesetClosure $item $src $variation "Weapon $($item.Id)"
        # 模型挂点行：决定主模型和 _1 挂在哪个假点（要配合 tools\ga_chr_dummy.ps1 加的假点）
        $absorpId = [int]$src["absorpParamId"].Value
        if (-not (Get-Row $nr.WepAbsorpPosParam $absorpId)) {
            $absorp = Add-Row $nr.WepAbsorpPosParam (Get-Row $nr.WepAbsorpPosParam ([int]$tpl["absorpParamId"].Value)) $absorpId "Weapon $($item.Id)"
            [void](Copy-Fields (Get-Row $ga.WepAbsorpPosParam $absorpId) $absorp)
        }
        # 战技行：招式由脚本接管，这里只要消耗和文本
        $artId = [int]$src["swordArtsParamId"].Value
        if (Get-Row $nr.SwordArtsParam $artId) { throw "SwordArtsParam $artId already exists" }
        $artSrc = Get-Row $ga.SwordArtsParam $artId
        $artTpl = Get-Row $nr.SwordArtsParam ([int]$tpl["swordArtsParamId"].Value)
        $art = Add-Row $nr.SwordArtsParam $artTpl $artId "Weapon $($item.Id)"
        [void](Copy-Delta $artSrc $art $ga.SwordArtsParam $nr.SwordArtsParam @{} @("iconId"))
        Set-Cell $art "iconId" $artTpl["iconId"].Value
        # 消耗恰好等于法环众数时 Copy-Delta 不写，这里明确带上
        [void](Copy-Fields $artSrc $art -Only @("useMagicPoint_L1", "useMagicPoint_L2", "useMagicPoint_R1", "useMagicPoint_R2"))
        Set-Cell $art "swordArtsType" $artSrc["swordArtsTypeNew"].Value
        $textPlan += @{ Id = [int]$artSrc["textId"].Value; Fmgs = $ArtText }
    } else {
        $atkTemplate = $null
        $behaviorTemplate = $nr.BehaviorParam_PC.Param.Rows[0]
        foreach ($b in $behaviors) {
            $new = Add-Row $nr.BehaviorParam_PC $behaviorTemplate $b.ID "Weapon $($item.Id)"
            [void](Copy-Fields $b $new)
            if ([int]$b["refType"].Value -ne $RefAttack) { throw "weapon $($item.Id): behavior $($b.ID) is not an attack; use Moveset for bullets" }
            $atkId = [int]$b["refId"].Value
            if (Get-Row $nr.AtkParam_Pc $atkId) { if (-not $atkTemplate) { $atkTemplate = Get-Row $nr.AtkParam_Pc $atkId }; continue }
            $added++
        }
        foreach ($b in $behaviors) {
            $atkId = [int]$b["refId"].Value
            if (Get-Row $nr.AtkParam_Pc $atkId) { continue }
            $srcAtk = Get-Row $ga.AtkParam_Pc $atkId
            if (-not $srcAtk) { throw "GA attack $atkId not found" }
            $newAtk = Add-Row $nr.AtkParam_Pc $atkTemplate $atkId "Weapon $($item.Id)"
            $delta = Copy-Delta $srcAtk $newAtk $ga.AtkParam_Pc $nr.AtkParam_Pc @{} @() $CopyAll
            if ($delta.Dropped.Count) { "  攻击 $atkId 丢弃（黑夜君临没有）: $($delta.Dropped -join ', ')" }
        }
    }
    # 模型（普通与低模）
    $model = [int]$src["equipModelId"].Value
    New-Item -ItemType Directory -Force "$pkg\parts" | Out-Null
    foreach ($suffix in "", "_l") {
        $name = "wp_a_{0:D4}$suffix.partsbnd.dcx" -f $model
        Copy-Item "$GaRoot\parts\$name" "$pkg\parts\$name" -Force
    }
    $icon = Use-Icon ([int]$src["iconId"].Value)
    Set-Cell $row "iconId" $icon
    $textPlan += @{ Id = $item.Id; Fmgs = $WeaponText }
    "武器 $($item.Id)：变体 $variation 判定 $($behaviors.Count) 行，模型 $model，图标 $($src['iconId'].Value) -> $icon，攻击 ×$([Math]::Round($scale, 3)) = $($row['attackBasePhysics'].Value)"
}

Add-VersatilityEffects
Add-ShownEffects
# GA 新增效果行的 guardStaminaMult 有上千行是 0，黑夜君临里等于格挡不掉精力，不带过来
foreach ($r in $nr.SpEffectParam.Param.Rows) {
    if ($r.Name -and $r.Name.StartsWith($NamePrefix) -and [double]$r["guardStaminaMult"].Value -eq 0) { Set-Cell $r "guardStaminaMult" 1 }
}
foreach ($r in $nr.AtkParam_Pc.Param.Rows) {
    if ($r.Name -and $r.Name.StartsWith($NamePrefix)) { Set-ReviveCorrection $r }
}

# 行大小变了说明有字段被截断或多写，游戏按固定结构读，全表都会错位读数
foreach ($t in $nr.Keys) {
    [byte[]]$written = $nr[$t].Param.Write()
    $after = [SoulsFormats.PARAM]::Read($written).DetectedSize
    if ($after -ne $nr[$t].Param.DetectedSize) { throw "$t row size changed: $($nr[$t].Param.DetectedSize) -> $after" }
    $nr[$t].File.Bytes = $written
}
Invoke-InDeps { [SoulsFormats.SFUtil]::EncryptNightreignRegulation($regPath, $nrBnd) }

# ---- 文本 ----
foreach ($lang in $Langs) {
    $gaMsg = Read-Bnd "$GaRoot\msg\$lang\item_dlc02.msgbnd.dcx"
    $gaFmg = @{}
    foreach ($f in $gaMsg.Files) { $gaFmg[[int]$f.ID] = [SoulsFormats.FMG]::Read($f.Bytes) }
    $target = "$pkg\msg\$lang\item_dlc01.msgbnd.dcx"
    $msg = Read-Bnd $target
    $count = 0
    $plan = @($textPlan | ForEach-Object { $p = $_; $p.Fmgs | ForEach-Object { @{ Id = $p.Id; From = $_.From; To = $_.To } } }) + $ownText
    foreach ($group in ($plan | Group-Object { $_.To })) {
        $file = $msg.Files | Where-Object { $_.ID -eq [int]$group.Name } | Select-Object -First 1
        if (-not $file) { throw "$lang FMG $($group.Name) not found" }
        $fmg = [SoulsFormats.FMG]::Read($file.Bytes)
        foreach ($t in $group.Group) {
            $text = $t[$lang]
            if (-not $text) {
                foreach ($from in $t.From) { $e = $gaFmg[[int]$from].Entries | Where-Object { $_.ID -eq $t.Id -and $_.Text } | Select-Object -First 1; if ($e) { $text = $e.Text; break } }
            }
            if (-not $text) { continue }
            if ($fmg.Entries | Where-Object { $_.ID -eq $t.Id }) { throw "$lang FMG $($group.Name) entry $($t.Id) already used" }
            $fmg.Entries.Add((New-Object SoulsFormats.FMG+Entry($fmg, $t.Id, $text)))
            $count++
        }
        $sorted = @($fmg.Entries | Sort-Object ID)
        $fmg.Entries.Clear()
        foreach ($e in $sorted) { $fmg.Entries.Add($e) }
        $file.Bytes = [byte[]]$fmg.Write()
    }
    Write-Bnd $msg $target
    "$lang\item_dlc01：新增文本 $count 条"
}
Write-StatusText $pkg $Langs

# ---- 图标：搬进黑夜君临图集的空白处 ----
# TPF 格式号两边都是 102，但 GA 是 BC7（DDS 头带 DX10 扩展，148 字节），黑夜君临的图集实际是 DXT5（128 字节），块不能直接拷，要转码
function Get-DdsHeaderSize([byte[]]$Dds) {
    if ([Text.Encoding]::ASCII.GetString($Dds, 84, 4) -eq "DX10") { 148 } else { 128 }
}
# 从 GA 图集裁出一个图标存成小 BC7 DDS，转成 DXT5，返回 40x40 个 4x4 块的字节
function Get-IconDxt5([byte[]]$Src, $Sub) {
    $header = Get-DdsHeaderSize $Src
    if ($header -ne 148) { throw "GA atlas is not BC7" }
    $blocks = $IconPx / 4
    $row = [BitConverter]::ToInt32($Src, 16) / 4
    $small = New-Object byte[] ($header + $blocks * $blocks * $BlockBytes)
    [Array]::Copy($Src, $small, $header)
    [BitConverter]::GetBytes([int]$IconPx).CopyTo($small, 12)
    [BitConverter]::GetBytes([int]$IconPx).CopyTo($small, 16)
    [BitConverter]::GetBytes([int]($blocks * $BlockBytes)).CopyTo($small, 20)
    for ($r = 0; $r -lt $blocks; $r++) {
        [Array]::Copy($Src, $header + (($Sub.Y / 4 + $r) * $row + $Sub.X / 4) * $BlockBytes, $small, $header + $r * $blocks * $BlockBytes, $blocks * $BlockBytes)
    }
    $tmp = [IO.Path]::GetTempFileName()
    $in = "$tmp.bc7.dds"; $out = "$tmp.dxt5.dds"
    try {
        [IO.File]::WriteAllBytes($in, $small)
        & python -I "$PSScriptRoot\ga_icon_dxt5.py" $in $out
        if ($LASTEXITCODE -ne 0) { throw "icon transcode failed (needs Pillow)" }
        [byte[]]$dds = [IO.File]::ReadAllBytes($out)
        if ([Text.Encoding]::ASCII.GetString($dds, 84, 4) -ne "DXT5") { throw "transcoded icon is not DXT5" }
        $bytes = New-Object byte[] ($blocks * $blocks * $BlockBytes)
        [Array]::Copy($dds, 128, $bytes, 0, $bytes.Length)
        , $bytes
    } finally { Remove-Item $tmp, $in, $out -ErrorAction SilentlyContinue }
}
function Get-Subs([string]$Text) {
    $map = @{}
    foreach ($m in [regex]::Matches($Text, '<SubTexture\s[^>]*/>')) {
        $a = @{}
        foreach ($kv in [regex]::Matches($m.Value, '(\w+)="([^"]*)"')) { $a[$kv.Groups[1].Value] = $kv.Groups[2].Value }
        $map[$a.name] = @{ X = [int]$a.x; Y = [int]$a.y; W = [int]$a.width; H = [int]$a.height }
    }
    $map
}
Invoke-InDeps {
    $gaTpf = [SoulsFormats.TPF]::Read("$GaRoot\menu\hi\01_common.tpf.dcx")
    $gaLay = [SoulsFormats.BND4]::Read("$GaRoot\menu\hi\01_common.sblytbnd.dcx")
    $tpf = [SoulsFormats.TPF]::Read("$pkg\menu\01_common_h.tpf.dcx")
    $lay = [SoulsFormats.BND4]::Read("$pkg\menu\01_common_h.sblytbnd.dcx")
    $layFile = $lay.Files | Where-Object { [IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $IconAtlas } | Select-Object -First 1
    $dstTex = $tpf.Textures | Where-Object { $_.Name -eq $IconAtlas } | Select-Object -First 1
    [byte[]]$dst = $dstTex.Bytes
    $text = [Text.Encoding]::UTF8.GetString($layFile.Bytes.ToArray())
    $srcLayouts = @{}
    foreach ($f in $gaLay.Files) { $srcLayouts[[IO.Path]::GetFileNameWithoutExtension($f.Name)] = [Text.Encoding]::UTF8.GetString($f.Bytes.ToArray()) }
    foreach ($icon in $icons) {
        $srcName = "MENU_ItemIcon_{0:D5}.png" -f $icon.Source
        $name = "MENU_ItemIcon_{0:D5}.png" -f $icon.New
        $atlas = $srcLayouts.Keys | Where-Object { $srcLayouts[$_].Contains("name=""$srcName""") } | Select-Object -First 1
        if (-not $atlas) { throw "icon $srcName not found in GA" }
        $s = (Get-Subs $srcLayouts[$atlas])[$srcName]
        $srcTex = $gaTpf.Textures | Where-Object { $_.Name -eq $atlas } | Select-Object -First 1
        [byte[]]$src = $srcTex.Bytes
        if ($s.X % 4 -or $s.Y % 4) { throw "icon $name`: not block aligned" }
        if ([Text.Encoding]::ASCII.GetString($dst, 84, 4) -ne "DXT5") { throw "$IconAtlas is not DXT5" }
        [byte[]]$iconBlocks = Get-IconDxt5 $src $s
        # 找一个不压任何已有子图的位置
        $used = Get-Subs $text
        $spot = $null
        for ($y = $AtlasPx - $IconPx - 4; $y -ge 0 -and -not $spot; $y -= $IconStep) {
            for ($x = 4; $x -le $AtlasPx - $IconPx; $x += $IconStep) {
                $hit = $false
                foreach ($u in $used.Values) { if ($x -lt $u.X + $u.W -and $u.X -lt $x + $IconPx -and $y -lt $u.Y + $u.H -and $u.Y -lt $y + $IconPx) { $hit = $true; break } }
                if (-not $hit) { $spot = @{ X = $x - $x % 4; Y = $y - $y % 4 }; break }
            }
        }
        if (-not $spot) { throw "no free slot in $IconAtlas for $name" }
        $dstHeader = Get-DdsHeaderSize $dst
        $dstRow = [BitConverter]::ToInt32($dst, 16) / 4
        $blocks = $IconPx / 4
        for ($r = 0; $r -lt $blocks; $r++) {
            [Array]::Copy($iconBlocks, $r * $blocks * $BlockBytes, $dst, $dstHeader + (($spot.Y / 4 + $r) * $dstRow + $spot.X / 4) * $BlockBytes, $blocks * $BlockBytes)
        }
        $line = "<SubTexture name=""$name"" x=""$($spot.X)"" width=""$IconPx"" y=""$($spot.Y)"" height=""$IconPx"" originalWidth=""$IconPx"" originalHeight=""$IconPx"" half=""0""/>"
        $nl = if ($text.Contains("`r`n")) { "`r`n" } else { "`n" }
        $close = $text.LastIndexOf("</TextureAtlas>")
        $text = $text.Substring(0, $close) + "`t" + $line + $nl + $text.Substring($close)
        "图标 $name <- $srcName $atlas ($($s.X),$($s.Y)) -> $IconAtlas ($($spot.X),$($spot.Y))"
    }
    $dstTex.Bytes = $dst
    $layFile.Bytes = [Text.Encoding]::UTF8.GetBytes($text)
    Add-StatusIcons $gaTpf $gaLay $tpf $lay
    $tpf.Write("$pkg\menu\01_common_h.tpf.dcx")
    $lay.Write("$pkg\menu\01_common_h.sblytbnd.dcx")
}

# ---- 自检：写出去的文件读回来核对 ----
Invoke-InDeps { $script:check = [SoulsFormats.SFUtil]::DecryptNightreignRegulation($regPath) }
$acc = Open-Param $check "EquipParamAccessory" "$Defs\NR\Defs\EquipParamAccessory.xml"
$wep = Open-Param $check "EquipParamWeapon" "$Defs\NR\Defs\EquipParamWeapon.xml"
$cus = Open-Param $check "EquipParamCustomWeapon" "$Defs\NR\Defs\EquipParamCustomWeapon.xml"
$eff = Open-Param $check "SpEffectParam" "$Defs\NR\Defs\SpEffectParam.xml"
$att = Open-Param $check "AttachEffectParam" "$Defs\NR\Defs\AttachEffectParam.xml"
foreach ($item in $Talismans) {
    $r = Get-Row $acc $item.Id
    if (-not $r) { throw "self-check: talisman $($item.Id) missing" }
    $a = Get-Row $att ([int]$r["spEffectId_1"].Value)
    if (-not $a) { throw "self-check: talisman $($item.Id) slot 1 is not an attach effect" }
    if (-not (Get-Row $eff ([int]$a["passiveSpEffectId_1"].Value))) { throw "self-check: talisman $($item.Id) effect missing" }
}
foreach ($item in $Weapons) {
    $r = Get-Row $wep $item.Id
    if (-not $r) { throw "self-check: weapon $($item.Id) missing" }
    if (-not (Get-Row $cus $item.Id)) { throw "self-check: weapon $($item.Id) has no custom weapon row" }
    if ([int]$r["wepmotionCategory"].Value -ne [int]$wep.Param.Rows[0]["wepmotionCategory"].Value -and -not (Test-Path ("$pkg\parts\wp_a_{0:D4}.partsbnd.dcx" -f [int]$r["equipModelId"].Value))) { throw "self-check: weapon $($item.Id) model missing" }
}
"ga_port 完成"
