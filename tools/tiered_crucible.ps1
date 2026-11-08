<#
  执行者8「熔炉妖刀」锻体带的绝招：熔炉形态（取代变野兽）。就地改 <Pkg>：
    regulation.bin  形态本体（减伤 40%、攻击 +10%、攻击与防御不耗精力、变身外观）、扣量 70 与持续扣量（每 1/6 秒 0.3，同原版野兽）、
                    霸体、给附近友方回血（发出效果 → 行为 → 子弹，2% / 执行者6 5%）、踏地时的蓄力黄金之怒（伤害敌人）、
                    变招之尾 / 之角的行为与攻击行、熔炉斧形套装 4 行防具
    parts           熔炉斧形套装模型（法环本体归档 hd / bd / am / lg_m_2110 与低模）
    msg             执行者8 / 4 / 6 的锻体文本按 tiered_texts.ps1 改写（中 / 英）
  脚本在 tiered_crucible.lua（tiered_hks.ps1 插入），执行者4 的锁血在 DLL（shared\src\tiered_beast.rs）。
  重复运行会报错（行已存在）；编号要与 lua、DLL 一致
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$ErGame = "D:\steam\steamapps\common\ELDEN RING\Game",
    [string]$Smithbox = "D:\relink\114514\smithbox",
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")
. (Join-Path $PSScriptRoot "fs_archive.ps1")
$NamePrefix = "[Tiered Buff] Executor: "; $ModeSample = 1500
. (Join-Path $PSScriptRoot "ga_params.ps1")
. (Join-Path $PSScriptRoot "tiered_texts.ps1")
$NrDefs = "$Smithbox\Assets\PARAM\NR\Defs"
$ErDefs = "$Smithbox\Assets\PARAM\ER\Defs"

$Form = 9971870; $Consume = 9971871; $Drain = 9971872; $Poise = 9971873
$Aura = 9971874; $AuraStrong = 9971875; $Regen = 9971876; $RegenStrong = 9971877; $AuraCooldown = 9971878; $Active = 9971879
$VfxParts = 9971870; $VfxFullBody = 9971871
$AuraBullet = 900997109; $AuraStrongBullet = 900997110
# 踏地时放的蓄力黄金之怒（tiered_crucible_anim.ps1 与骑士爆发同时挂 $Wrath）：发出链同金8 冲击波（relics\src\tiered\dll.rs），
# 抄 3 级振奋香追加爆炸（发出效果 708696、行为 760000110、子弹 13350050）；子弹换成蓄力黄金之怒 10641050 的特效与范围，
# 攻击行抄效果型「黄金冲击波」12064105（只打敌人、击退、削韧），圣属性取蓄力版 64105 的值；绝招自己的伤害不回绝招量表
$Wrath = 9971880; $WrathBullet = 900997129
$WrathLook = "sfxId_Bullet", "sfxId_Hit", "life", "hitRadius", "hitRadiusMax", "spreadTime"
# 熔炉形态变招（tiered_crucible_arts.ps1 的动作用全局判定 699x 打这几行）：伤害按妖刀算（底子是妖刀轻攻击 900300 / 行为 100900300，倍率另给），
# 判定球（尾巴挂点 820~824 / 角挂点 7→1）、击退、削韧、受击等级、命中效果照原版法术攻击行。
# 行为要抄妖刀自己的（类别 12 = 技能武器攻击）：执行者7 的全局行为类别是 0，只挂效果、不算武器攻击力，抄它伤害是 0。
# 判定来源保持模板的 0（挂点号 1000 以内 = 身体、10xxx / 11xxx = 左右手武器、5xxx = 妖刀）：法术攻击行的 1 是施法事件用的来源，攻击事件里找不到挂点
$ArtAtkTemplate = 900300; $ArtBehaviorTemplate = 100900300
$ArtLook = @("knockbackDist", "atkSuperArmor", "dmgLevel", "spEffectId0")
$Arts = @(
    @{ Judge = 6990; Atk = 900997130; Spell = 75000; Rate = 150; Name = "Crucible form - Tail" },
    @{ Judge = 6991; Atk = 900997131; Spell = 75005; Rate = 170; Name = "Crucible form - Tail (charged 2nd sweep)" },
    @{ Judge = 6992; Atk = 900997132; Spell = 75105; Rate = 260; Name = "Crucible form - Horns (charged)" }
)
$CorrectionFields = "atkPhysCorrection", "atkMagCorrection", "atkFireCorrection", "atkThunCorrection", "atkDarkCorrection"
$Armor = 570000; $ArmorModel = 2110
$AuraRadius = 10.0
$CutFields = "slash", "blow", "thrust", "neutral", "magic", "fire", "thunder", "dark" | ForEach-Object { "${_}DamageCutRate" }
$AttackFields = "physics", "magic", "fire", "thunder", "dark" | ForEach-Object { "${_}AttackRate" }
# 模板：707110 野兽不死（清掉状态 143）、6999611 加量表、102560 持续扣量、5300 全不硬直、7010280 / 900102221 / 7010221 遗物「圣杯瓶也能治疗队友」、
# 6999615 绝招中（当 1 秒计时）、1414980 / 1414981 魔人化的变身外观（普通 / 全身两行）、5050000 起黑夜君临变身防具
$TransformParts = @(@{ Offset = 0; Prefix = "hd" }, @{ Offset = 100; Prefix = "bd" }, @{ Offset = 200; Prefix = "am" }, @{ Offset = 300; Prefix = "lg" })
$TransformFields = @("equipModelId", "equipModelCategory", "equipModelGender", "headEquip", "bodyEquip", "armEquip", "legEquip", "isDeposit")
$TextIds = 9971865, 9971350, 9971770
$Languages = "zhocn", "engus"

function Set-Cells($Row, [hashtable]$Values) { foreach ($k in $Values.Keys) { Set-Cell $Row $k $Values[$k] } }

$pkgPath = (Resolve-Path $Pkg).Path
$regPath = "$pkgPath\regulation.bin"
Invoke-InDeps { $script:bnd = [SoulsFormats.SFUtil]::DecryptNightreignRegulation($regPath) }
$sp = Open-Param $bnd "SpEffectParam" "$NrDefs\SpEffectParam.xml"
$vfx = Open-Param $bnd "SpEffectVfxParam" "$NrDefs\SpEffectVfxParam.xml"
$beh = Open-Param $bnd "BehaviorParam_PC" "$NrDefs\BehaviorParam.xml"
$bul = Open-Param $bnd "Bullet" "$NrDefs\Bullet.xml"
$atk = Open-Param $bnd "AtkParam_Pc" "$NrDefs\AtkParam.xml"
$pro = Open-Param $bnd "EquipParamProtector" "$NrDefs\EquipParamProtector.xml"

# ---- 效果 ----
$formRow = Add-Row $sp (Get-Row $sp 707110) $Form "Crucible form"
foreach ($f in $CutFields) { Set-Cell $formRow $f 0.6 }
foreach ($f in $AttackFields) { Set-Cell $formRow $f 1.1 }
Set-Cells $formRow @{ stateInfo = 0; effectEndurance = -1; consumeStaminaRate = 0; guardStaminaMult = 0; vfxId = $VfxParts; vfxId1 = $VfxFullBody }
Set-Cells (Add-Row $sp (Get-Row $sp 6999611) $Consume "Crucible form - Consume gauge") @{ ultimateArtGauge = -70; replaceSpEffectId = -1; vfxId = -1 }
Set-Cells (Add-Row $sp (Get-Row $sp 102560) $Drain "Crucible form - Drain gauge") @{ effectEndurance = -1; motionInterval = (1.0 / 6); ultimateArtGauge = -0.3 }
Set-Cells (Add-Row $sp (Get-Row $sp 5300) $Poise "Crucible form - Poise") @{ effectEndurance = 0.5 }
Set-Cells (Add-Row $sp (Get-Row $sp 6999615) $AuraCooldown "Crucible form - Aura timer") @{ effectEndurance = 1.0 }
# 踏地中标记：按绝招挂、踏地完摘，DLL 见到它就把招式表切成熔炉妖刀（不用先拔刀就能播踏地）
Set-Cells (Add-Row $sp (Get-Row $sp 6999615) $Active "Crucible form - Active") @{ effectEndurance = -1 }
foreach ($a in @(@{ Spawn = $Aura; Bullet = $AuraBullet; Regen = $Regen; Rate = -2; Name = "Crucible form - Ally regen" },
                 @{ Spawn = $AuraStrong; Bullet = $AuraStrongBullet; Regen = $RegenStrong; Rate = -5; Name = "Crucible form - Ally regen (Roar buff)" })) {
    Set-Cells (Add-Row $sp (Get-Row $sp 7010221) $a.Regen "$($a.Name) - Heal") @{ changeHpEstusFlaskRate = 0; changeHpRate = $a.Rate; vfxId = -1 }
    Set-Cells (Add-Row $sp (Get-Row $sp 7010280) $a.Spawn "$($a.Name) - Spawn") @{ behaviorId = $a.Bullet }
    Set-Cells (Add-Row $beh (Get-Row $beh 900102221) $a.Bullet $a.Name) @{ refId = $a.Bullet }
    Set-Cells (Add-Row $bul (Get-Row $bul 900102221) $a.Bullet $a.Name) @{ hitRadius = $AuraRadius; spEffectId0 = $a.Regen }
}
Set-Cells (Add-Row $sp (Get-Row $sp 708696) $Wrath "Crucible form - Golden Wrath (Spawn)") @{ behaviorId = $WrathBullet; spCategory = 20; categoryPriority = 0 }
Set-Cells (Add-Row $beh (Get-Row $beh 760000110) $WrathBullet "Crucible form - Golden Wrath") @{ refId = $WrathBullet }
$wrathBulletRow = Add-Row $bul (Get-Row $bul 13350050) $WrathBullet "Crucible form - Golden Wrath"
[void](Copy-Fields (Get-Row $bul 10641050) $wrathBulletRow -Only $WrathLook)
Set-Cells $wrathBulletRow @{ atkId_Bullet = $WrathBullet }
Set-Cells (Add-Row $atk (Get-Row $atk 12064105) $WrathBullet "Crucible form - Golden Wrath") @{ atkDark = [int](Get-Row $atk 64105)["atkDark"].Value; ultimateChargeCorrection = 0 }
foreach ($a in $Arts) {
    $spell = Get-Row $atk $a.Spell
    $row = Add-Row $atk (Get-Row $atk $ArtAtkTemplate) $a.Atk $a.Name
    $hitFields = @($spell.Cells | ForEach-Object { $_.Def.InternalName } | Where-Object { $_ -match '^hit\d+_' })
    [void](Copy-Fields $spell $row -Only ($hitFields + $ArtLook))
    foreach ($f in $CorrectionFields) { Set-Cell $row $f $a.Rate }
    $behaviorId = 600000000 + $a.Judge % 1000
    Set-Cells (Add-Row $beh (Get-Row $beh $ArtBehaviorTemplate) $behaviorId $a.Name) @{ variationId = 0; behaviorJudgeId = $a.Judge % 1000; refId = $a.Atk }
}
"效果：熔炉形态 $Form 及扣量、霸体、队友回血 $Aura / $AuraStrong（子弹 $AuraBullet / $AuraStrongBullet，半径 $AuraRadius）、蓄力黄金之怒 $Wrath（行为 / 子弹 / 攻击 $WrathBullet）"

# ---- 变身外观与防具（法环本体熔炉斧形套装）----
Set-Cells (Add-Row $vfx (Get-Row $vfx 1414980) $VfxParts "Crucible form - Armor") @{ transformProtectorId = $Armor }
Set-Cells (Add-Row $vfx (Get-Row $vfx 1414981) $VfxFullBody "Crucible form - Armor (full body)") @{ transformProtectorId = $Armor }
Invoke-InDeps { $script:erBnd = [SoulsFormats.SFUtil]::DecryptERRegulation("$ErGame\regulation.bin") }
$erPro = Open-Param $erBnd "EquipParamProtector" "$ErDefs\EquipParamProtector.xml"
foreach ($part in $TransformParts) {
    $src = Get-Row $erPro ($Armor + $part.Offset)
    if (-not $src -or [int]$src["equipModelId"].Value -ne $ArmorModel) { throw "Elden Ring protector $($Armor + $part.Offset) is not model $ArmorModel" }
    $new = Add-Row $pro (Get-Row $pro (5050000 + $part.Offset)) ($Armor + $part.Offset) "Crucible form armor"
    $fields = $TransformFields + @($src.Cells | ForEach-Object { $_.Def.InternalName } | Where-Object { $_ -like "invisibleFlag_*" })
    [void](Copy-Fields $src $new -Only $fields)
}
New-Item -ItemType Directory -Force "$pkgPath\parts" | Out-Null
$er = Open-ErArchives $ErGame $Smithbox "$pkgPath\parts"
$files = 0
foreach ($part in $TransformParts) {
    foreach ($suffix in "", "_l") {
        $name = "{0}_m_{1}{2}.partsbnd.dcx" -f $part.Prefix, $ArmorModel, $suffix
        [byte[]]$bytes = Get-ErFile $er "/parts/$name"
        if (-not $bytes) { throw "$name not found in the Elden Ring archives" }
        [IO.File]::WriteAllBytes("$pkgPath\parts\$name", $bytes); $files++
    }
}
"变身防具：$Armor 起 4 行，模型 $ArmorModel 共 $files 个文件"

foreach ($t in $sp, $vfx, $beh, $bul, $atk, $pro) {
    [byte[]]$bytes = $t.Param.Write()
    if ([SoulsFormats.PARAM]::Read($bytes).DetectedSize -ne $t.Param.DetectedSize) { throw "$($t.Param.ParamType) row size changed" }
    $t.File.Bytes = $bytes
}
Invoke-InDeps { [SoulsFormats.SFUtil]::EncryptNightreignRegulation($regPath, $bnd) }

# ---- 文本：改写已有条目（锻体名 / 说明、状态界面效果名）----
$texts = @($TieredTexts | Where-Object { $TextIds -contains $_.Id })
if ($texts.Count -ne $TextIds.Count) { throw "tiered_texts.ps1 misses some of $($TextIds -join ', ')" }
foreach ($lang in $Languages) {
    $zh = $lang -eq "zhocn"
    foreach ($target in @(@{ Bnd = "item_dlc01"; Fmgs = @{ 51 = "Name"; 52 = "Info" } }, @{ Bnd = "menu_dlc01"; Fmgs = @{ 214 = "Name" } })) {
        $path = "$pkgPath\msg\$lang\$($target.Bnd).msgbnd.dcx"
        $msg = Read-Bnd $path
        foreach ($fmgId in $target.Fmgs.Keys) {
            $file = $msg.Files | Where-Object { $_.ID -eq $fmgId } | Select-Object -First 1
            $fmg = [SoulsFormats.FMG]::Read($file.Bytes)
            foreach ($t in $texts) {
                $entry = $fmg.Entries | Where-Object { $_.ID -eq $t.Id } | Select-Object -First 1
                if (-not $entry) { throw "$lang $($target.Bnd) FMG $fmgId has no entry $($t.Id)" }
                $entry.Text = if ($target.Fmgs[$fmgId] -eq "Name") { if ($zh) { $t.Zh } else { $t.En } } else { if ($zh) { $t.ZhInfo } else { $t.EnInfo } }
            }
            $file.Bytes = [byte[]]$fmg.Write()
        }
        Write-Bnd $msg $path
    }
}
"文本：$($TextIds -join ' / ') 已按 tiered_texts.ps1 改写（$($Languages -join ' / ')）"

# 自检
Invoke-InDeps { $script:check = [SoulsFormats.SFUtil]::DecryptNightreignRegulation($regPath) }
$cs = Open-Param $check "SpEffectParam" "$NrDefs\SpEffectParam.xml"
$cf = Get-Row $cs $Form
if (-not $cf -or [int]$cf["stateInfo"].Value -ne 0 -or [int]$cf["vfxId"].Value -ne $VfxParts) { throw "self-check: crucible form row wrong" }
foreach ($id in $Consume, $Drain, $Poise, $Aura, $AuraStrong, $Regen, $RegenStrong, $AuraCooldown, $Active, $Wrath) { if (-not (Get-Row $cs $id)) { throw "self-check: effect $id missing" } }
$cb = Get-Row (Open-Param $check "Bullet" "$NrDefs\Bullet.xml") $WrathBullet
if (-not $cb -or [int]$cb["sfxId_Bullet"].Value -ne 525316 -or [int]$cb["atkId_Bullet"].Value -ne $WrathBullet) { throw "self-check: golden wrath bullet wrong" }
$cbeh = Open-Param $check "BehaviorParam_PC" "$NrDefs\BehaviorParam.xml"
$catk = Open-Param $check "AtkParam_Pc" "$NrDefs\AtkParam.xml"
foreach ($a in $Arts) {
    $b = Get-Row $cbeh (600000000 + $a.Judge % 1000); $r = Get-Row $catk $a.Atk
    if (-not $b -or [int]$b["refId"].Value -ne $a.Atk -or [int]$b["category"].Value -ne [int](Get-Row $cbeh $ArtBehaviorTemplate)["category"].Value -or
        -not $r -or [int]$r["atkPhysCorrection"].Value -ne $a.Rate -or [int]$r["hitSourceType"].Value -ne 0) { throw "self-check: art $($a.Name) rows wrong" }
}
"tiered_crucible 完成：regulation " + (Get-FileHash $regPath).Hash.Substring(0, 8)
