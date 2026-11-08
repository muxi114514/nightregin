<#
  招式类武器的参数闭包（ga_port.ps1 点引用，用它的 Add-Row / Copy-Delta 和 $nr / $ga / $er 表）。
  从判定行、招式表事件和指定的效果出发，把引用到的攻击、子弹、效果、外观行都带过来。
  法环原版也有的行直接用黑夜君临的；mod 新增的行如果在黑夜君临已被占用就报错，不能覆盖别的内容。
#>
$RefBullet = 1
$RefEffect = 2
$TaeAttack = 1
$TaeBullet = 2
$TaeEffects = 66, 67
$JudgeOffset = 8
$EffectTemplate = 310000
# 这两个字段黑夜君临几乎所有子弹都是 -65 / -128，法环都是 0，含义未确认，先留黑夜君临的常用值
$BulletKeepNr = @("knockbackDamp", "staminaDamp")
# 变身防具：模板是追踪者的深渊行者皮肤（同样按全身变身用）
$TransformTemplate = 5050000
$TransformParts = @(@{ Offset = 0; Prefix = "hd" }, @{ Offset = 100; Prefix = "bd" }, @{ Offset = 200; Prefix = "am" }, @{ Offset = 300; Prefix = "lg" })
$TransformFields = @("equipModelId", "equipModelCategory", "equipModelGender", "headEquip", "bodyEquip", "armEquip", "legEquip", "isDeposit", "residentSpEffectId", "residentSpEffectId2", "residentSpEffectId3")

function Get-FieldNames($Table, [string]$Pattern) {
    @($Table.Param.AppliedParamdef.Fields | Where-Object { $_.InternalName -match $Pattern } | ForEach-Object { $_.InternalName })
}
function Get-RefIds($Row, [string[]]$Fields) {
    foreach ($f in $Fields) {
        $v = [int64]$Row[$f].Value
        if ($v -gt 0) { [int]$v }
    }
}
# 取招式表用到的判定号和效果号
function Read-TaeRefs([string[]]$Names) {
    $bnd = Read-Bnd "$GaRoot\chr\c0000.anibnd.dcx"
    $judges = @{}; $effects = @{}
    foreach ($name in $Names) {
        $tae = [SoulsFormats.TAE]::Read((Find-File $bnd "\$name.tae").Bytes)
        foreach ($anim in $tae.Animations) {
            foreach ($e in $anim.Events) {
                $type = [int]$e.Type
                $p = Get-Params $e
                if ($type -eq $TaeAttack -or $type -eq $TaeBullet) { $judges[[BitConverter]::ToInt32($p, $JudgeOffset)] = 1 }
                elseif ($TaeEffects -contains $type) { $effects[[BitConverter]::ToInt32($p, 0)] = 1 }
            }
        }
    }
    @{ Judges = $judges; Effects = $effects }
}
# 把一行带过来：已有就用现成的（mod 新增却撞号则报错），返回是否新增
function Copy-Row([string]$Table, [int]$Id, $Template, [string]$Label, [hashtable]$Alias, [string[]]$Skip, [string]$Always, $Dropped) {
    if (Get-Row $nr[$Table] $Id) {
        if (-not (Get-Row $er[$Table] $Id)) { throw "$Table $Id is used by the mod package and by GoldenAge" }
        return $null
    }
    $src = Get-Row $ga[$Table] $Id
    if (-not $src) { return $null }
    $new = Add-Row $nr[$Table] $Template $Id $Label
    $delta = Copy-Delta $src $new $ga[$Table] $nr[$Table] $Alias $Skip $Always
    foreach ($d in $delta.Dropped) { $Dropped[$d.Split('=')[0]] = 1 + [int]$Dropped[$d.Split('=')[0]] }
    $new
}

# 变身防具：外观行指向头部行，全身变身时 +100 身体、+200 手臂、+300 腿都要有。
# 数值沿用黑夜君临自己的皮肤行，带过来的只有模型和隐藏部位标记；模型文件（含低模）拷进 parts
function Copy-Transform([int]$Id, [string]$Label) {
    New-Item -ItemType Directory -Force "$pkg\parts" | Out-Null
    foreach ($part in $TransformParts) {
        $src = Get-Row $ga.EquipParamProtector ($Id + $part.Offset)
        if (-not $src) { throw "GA protector $($Id + $part.Offset) not found" }
        $new = Add-Row $nr.EquipParamProtector (Get-Row $nr.EquipParamProtector ($TransformTemplate + $part.Offset)) ($Id + $part.Offset) $Label
        $fields = $TransformFields + @($src.Cells | ForEach-Object { $_.Def.InternalName } | Where-Object { $_ -like "invisibleFlag_*" })
        [void](Copy-Fields $src $new -Only $fields)
        foreach ($suffix in "", "_l") {
            $name = "{0}_m_{1}{2}.partsbnd.dcx" -f $part.Prefix, [int]$src["equipModelId"].Value, $suffix
            Copy-Item "$GaRoot\parts\$name" "$pkg\parts\$name" -Force
        }
    }
    "  变身防具 $Id：模型 $([int](Get-Row $ga.EquipParamProtector $Id)['equipModelId'].Value)，4 行 + 8 个模型文件"
}

function Copy-MovesetClosure($Item, $Weapon, [int]$Variation, [string]$Label) {
    $lo = $BehaviorBase + $Variation * $VariationStep
    $refs = Read-TaeRefs $Item.Taes
    $qBullet = New-Object System.Collections.Queue
    $qEffect = New-Object System.Collections.Queue
    $attacks = @{}
    foreach ($k in $refs.Effects.Keys) { $qEffect.Enqueue([int]$k) }
    foreach ($id in $Item.Effects) { $qEffect.Enqueue([int]$id) }
    foreach ($id in Get-RefIds $Weapon @("residentSpEffectId", "residentSpEffectId1", "residentSpEffectId2")) { $qEffect.Enqueue($id) }

    # 判定行
    $behaviorTemplate = $nr.BehaviorParam_PC.Param.Rows[0]
    $behaviors = @($ga.BehaviorParam_PC.Param.Rows | Where-Object { $_.ID -ge $lo -and $_.ID -lt $lo + $VariationStep })
    foreach ($b in $behaviors) {
        $new = Add-Row $nr.BehaviorParam_PC $behaviorTemplate $b.ID $Label
        [void](Copy-Fields $b $new)
        $target = [int]$b["refId"].Value
        if ($target -le 0) { continue }
        switch ([int]$b["refType"].Value) {
            $RefAttack { $attacks[$target] = 1 }
            $RefBullet { $qBullet.Enqueue($target) }
            $RefEffect { $qEffect.Enqueue($target) }
        }
    }
    # 招式表用到、这个变体段没有的判定：黑夜君临的通用判定（变体 0）里要有，否则报出来
    $missing = @($refs.Judges.Keys | Where-Object { -not (Get-Row $nr.BehaviorParam_PC ($lo + [int]$_)) -and -not (Get-Row $nr.BehaviorParam_PC ($BehaviorBase + [int]$_)) })
    if ($missing.Count) { "  招式表的判定两边都没有行: $(($missing | Sort-Object) -join ' ')" }

    $fields = @{
        BulletBullet = Get-FieldNames $ga.Bullet '(?i)bullet_?id$'
        BulletEffect = Get-FieldNames $ga.Bullet '(?i)speffectid'
        BulletAttack = Get-FieldNames $ga.Bullet '(?i)^atkid'
        AttackEffect = Get-FieldNames $ga.AtkParam_Pc '(?i)speffectid'
        EffectEffect = Get-FieldNames $ga.SpEffectParam '(?i)speffectid$|^accumu(Over|Under)FireId$|^applyIdOnGetSoul$'
        EffectVfx = Get-FieldNames $ga.SpEffectParam '^vfxId\d?$'
    }
    # 定义没覆盖的尾部字段不会被重置，模板要选这段全零的行（绝大多数子弹都是全零，第 0 行不是）
    $bulletTemplate = $nr.Bullet.Param.Rows | Where-Object { ([byte[]]$_["tailPadding"].Value | Measure-Object -Maximum).Maximum -eq 0 } | Select-Object -First 1
    $vfxTemplate = $nr.SpEffectVfxParam.Param.Rows[0]
    $effectTemplate = Get-Row $nr.SpEffectParam $EffectTemplate
    $atkTemplate = $nr.AtkParam_Pc.Param.Rows[0]
    $seen = @{ Bullet = @{}; Effect = @{}; Attack = @{}; Vfx = @{} }
    $added = @{ Attack = 0; Bullet = 0; Effect = 0; Vfx = 0 }
    $dropped = @{}
    $absent = @{ Attack = 0; Bullet = 0; Effect = 0; Vfx = 0 }

    do {
        while ($qBullet.Count) {
            $id = $qBullet.Dequeue()
            if ($seen.Bullet.ContainsKey($id)) { continue }
            $seen.Bullet[$id] = 1
            $src = Get-Row $ga.Bullet $id
            if (-not $src) { $absent.Bullet++; continue }
            foreach ($v in Get-RefIds $src $fields.BulletBullet) { $qBullet.Enqueue($v) }
            foreach ($v in Get-RefIds $src $fields.BulletEffect) { $qEffect.Enqueue($v) }
            foreach ($v in Get-RefIds $src $fields.BulletAttack) { $attacks[$v] = 1 }
            if (Copy-Row "Bullet" $id $bulletTemplate $Label @{} $BulletKeepNr $CopyAll $dropped) { $added.Bullet++ }
        }
        foreach ($id in @($attacks.Keys)) {
            if ($seen.Attack.ContainsKey($id)) { continue }
            $seen.Attack[$id] = 1
            $src = Get-Row $ga.AtkParam_Pc $id
            if (-not $src) { $absent.Attack++; continue }
            foreach ($v in Get-RefIds $src $fields.AttackEffect) { $qEffect.Enqueue($v) }
            if (Copy-Row "AtkParam_Pc" $id $atkTemplate $Label @{} @() $CopyAll $dropped) { $added.Attack++ }
        }
        while ($qEffect.Count) {
            $id = $qEffect.Dequeue()
            if ($seen.Effect.ContainsKey($id)) { continue }
            $seen.Effect[$id] = 1
            $src = Get-Row $ga.SpEffectParam $id
            if (-not $src) { $absent.Effect++; continue }
            foreach ($v in Get-RefIds $src $fields.EffectEffect) { $qEffect.Enqueue($v) }
            foreach ($v in Get-RefIds $src $fields.EffectVfx) { $seen.Vfx[$v] = 1 }
            $new = Copy-Row "SpEffectParam" $id $effectTemplate $Label $EffectAlias @("iconId") $EffectAlways $dropped
            if ($new) { Clear-EffectUi $new; $added.Effect++ }
        }
    } while ($qBullet.Count -or $qEffect.Count -or @($attacks.Keys | Where-Object { -not $seen.Attack.ContainsKey($_) }).Count)

    foreach ($id in @($seen.Vfx.Keys)) {
        $new = Copy-Row "SpEffectVfxParam" $id $vfxTemplate $Label @{} @() $CopyAll $dropped
        if ($new) {
            $added.Vfx++
            # 没声明带过来的变身防具不留，免得引用不存在的防具
            if ($Item.Transforms -notcontains [int]$new["transformProtectorId"].Value) {
                Set-Cell $new "transformProtectorId" -1
                Set-Cell $new "isFullBodyTransformProtectorId" 0
            }
        }
    }
    foreach ($id in $Item.Transforms) { Copy-Transform $id $Label }
    "  判定$($behaviors.Count) 行；新增 攻击 $($added.Attack) 子弹 $($added.Bullet) 效果 $($added.Effect) 外观 $($added.Vfx)；引用了但 GA 也没有: 攻击 $($absent.Attack) 子弹 $($absent.Bullet) 效果 $($absent.Effect)"
    if ($dropped.Count) { "  黑夜君临没有的字段（丢弃）: " + (($dropped.GetEnumerator() | Sort-Object Value -Descending | ForEach-Object { "$($_.Key)×$($_.Value)" }) -join ', ') }
}
