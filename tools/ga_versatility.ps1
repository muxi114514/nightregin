<#
  多才多艺（GoldenAge 的招式种类加成）：20 秒内用过的招式种类越多攻击力越高，不分武器，脚本见 tools\ga_versatility.lua。
  这里是状态表和参数行：ga_hks.ps1 用状态表给攻击状态插标记，ga_port.ps1 调 Add-VersatilityEffects 加效果行、Add-StatusIcons 搬层数图标。
#>
$VersatilityBase = 103200
# 下标 + 基址 = 这个状态的标记效果（GA 的编号）。法环的潜行攻击状态黑夜君临的行为图没有，留空
$VersatilityStates = @(
    "AttackRightLight1", "AttackRightLight2", "AttackRightLight3", "AttackRightLight4", "AttackRightLight5", "AttackRightLight6",
    "AttackRightLightStep", "", "AttackRightLightFastStep", "AttackRightLightDash", "AttackRightHeavyDash", "AttackRightWhileGuard",
    "AttackRightHeavy1Start", "AttackRightHeavy1End", "AttackRightHeavy2Start", "AttackRightHeavy2End",
    "AttackRightHeavySpecial1Start", "AttackRightHeavySpecial1End", "AttackRightHeavySpecial2Start", "AttackRightHeavySpecial2End",
    "AttackRightSpecial1", "AttackRightSpecial2", "AttackRightBackstep",
    "AttackDualLight1", "AttackDualLight2", "AttackDualLight3", "AttackDualLight4", "AttackDualLight5", "AttackDualLight6",
    "AttackDualDash", "AttackDualRolling", "", "AttackDualBackStep",
    "AttackBothLight1", "AttackBothLight2", "AttackBothLight3", "AttackBothLight4", "AttackBothLight5", "AttackBothLight6",
    "AttackBothHeavy1Start", "AttackBothHeavy1End", "AttackBothHeavy2Start", "AttackBothHeavy2End",
    "AttackBothHeavySpecial1Start", "AttackBothHeavySpecial1End", "AttackBothHeavySpecial2Start", "AttackBothHeavySpecial2End",
    "AttackBothSpecial1", "AttackBothSpecial2", "AttackBothDash", "AttackBothHeavyDash", "AttackBothLightStep", "", "AttackBothBackstep",
    "AttackRightLightCounter", "AttackRightHeavyCounter", "AttackBothLightCounter", "AttackBothHeavyCounter"
)
$VersatilityMarkerSeconds = 20
# 层数效果要上状态栏，不能不限时（见 ga_status.ps1）；一局用不完这么久，由脚本主动移除，到期了脚本也会马上补挂
$VersatilityLayerSeconds = 3600
# 层数效果 → 攻击倍率。GA 每层 +2.75%、封顶 +25%，这里削成每层 +2%、封顶 +18%
$VersatilityLayers = @{ 103260 = 1.02; 103261 = 1.04; 103262 = 1.06; 103263 = 1.08; 103264 = 1.10; 103265 = 1.12; 103266 = 1.14; 103268 = 1.16; 103270 = 1.18 }
$VersatilityUnlock = 103269
# 0 层只显示不加成。GA 没有这一层：103267 是它层数段里空着的号，图标由 1 层改字（ga_icon_zero.py）
$VersatilityZeroLayer = 103267
$VersatilityZeroIcon = 20990
$VersatilityOneIcon = 20536
$VersatilityRates = "physicsAttackRate", "magicAttackRate", "fireAttackRate", "thunderAttackRate", "darkAttackRate"
# 状态图标：世界之眼 + 1~9 层。两边都是 BC7（DDS 头 148 字节），40x40 按块直接拷；黑夜君临图集里本来就有 40x40 的状态图标
$StatusIconIds = @(20533, 20536, 20537, 20538, 20539, 20540, 20541, 20542, 20543, 20544)
$StatusAtlas = "SB_StatusIcon_0"
$StatusIconPx = 40
$StatusIconStep = 44
$Bc7Header = 148
$Bc7BlockBytes = 16

# 行已存在（招式表或物品带过来的）就用现成的
function Get-VersatilityRow([int]$Id, $Template, [hashtable]$Alias, [string]$Label, $Dropped) {
    $row = Get-Row $nr.SpEffectParam $Id
    if (-not $row) { $row = Copy-Row "SpEffectParam" $Id $Template $Label $Alias @("iconId") $EffectAlways $Dropped }
    if (-not $row) { throw "GA effect $Id not found" }
    $row
}
function Add-VersatilityEffects {
    $template = Get-Row $nr.SpEffectParam $EffectTemplate
    $dropped = @{}
    for ($i = 0; $i -lt $VersatilityStates.Count; $i++) {
        if (-not $VersatilityStates[$i]) { continue }
        $row = Get-VersatilityRow ($VersatilityBase + $i) $template @{} "Versatility marker" $dropped
        Clear-EffectUi $row
        Set-Cell $row "effectEndurance" $VersatilityMarkerSeconds
    }
    $layer = 0
    foreach ($id in @($VersatilityLayers.Keys | Sort-Object)) {
        $layer++
        $percent = [Math]::Round(($VersatilityLayers[$id] - 1) * 100)
        $row = Get-VersatilityRow $id $template @{} "Versatility layer" $dropped
        Clear-EffectUi $row
        $source = Get-Row $ga.SpEffectParam $id
        Set-StatusShown $row ([int]$source["iconId"].Value) @{ zhocn = "多才多艺 $layer 层：提升 $percent% 攻击力"; engus = "Versatility $layer`: Attack +$percent%" } @{ zhocn = "多才多艺带来的效果"; engus = "Effect of Versatility" }
        Set-Cell $row "effectEndurance" $VersatilityLayerSeconds
        foreach ($f in $VersatilityRates) { Set-Cell $row $f $VersatilityLayers[$id] }
    }
    $zero = Add-Row $nr.SpEffectParam (Get-Row $nr.SpEffectParam ($VersatilityLayers.Keys | Sort-Object | Select-Object -First 1)) $VersatilityZeroLayer "Versatility layer"
    Set-StatusShown $zero $VersatilityZeroIcon @{ zhocn = "多才多艺 0 层：暂无攻击力加成"; engus = "Versatility 0: No attack bonus" } @{ zhocn = "多才多艺带来的效果"; engus = "Effect of Versatility" }
    foreach ($f in $VersatilityRates) { Set-Cell $zero $f 1 }
    [void](Get-VersatilityRow $VersatilityUnlock $template @{} "Versatility unlock" $dropped)
    "多才多艺：标记 $(@($VersatilityStates | Where-Object { $_ }).Count) 行，层数 $($VersatilityLayers.Count) 行 + 0 层"
}
# 图集空白处，4 对齐、不压已有子图
function Find-StatusSpot([string]$Text, [int]$Width, [int]$Height) {
    $used = Get-Subs $Text
    for ($y = $Height - $StatusIconPx - 4; $y -ge 0; $y -= $StatusIconStep) {
        for ($x = 4; $x -le $Width - $StatusIconPx; $x += $StatusIconStep) {
            $sx = $x - $x % 4; $sy = $y - $y % 4
            $hit = $false
            foreach ($u in $used.Values) { if ($sx -lt $u.X + $u.W -and $u.X -lt $sx + $StatusIconPx -and $sy -lt $u.Y + $u.H -and $u.Y -lt $sy + $StatusIconPx) { $hit = $true; break } }
            if (-not $hit) { return @{ X = $sx; Y = $sy } }
        }
    }
    throw "no free slot in $StatusAtlas"
}
# 按行取一个图标的 BC7 块（40x40 = 10 行，每行 10 块）
function Get-StatusIconBlocks([byte[]]$Atlas, $Rect) {
    $blocks = $StatusIconPx / 4
    $row = [BitConverter]::ToInt32($Atlas, 16) / 4
    [byte[]]$out = New-Object byte[] ($blocks * $blocks * $Bc7BlockBytes)
    for ($r = 0; $r -lt $blocks; $r++) {
        [Array]::Copy($Atlas, $Bc7Header + (($Rect.Y / 4 + $r) * $row + $Rect.X / 4) * $Bc7BlockBytes, $out, $r * $blocks * $Bc7BlockBytes, $blocks * $Bc7BlockBytes)
    }
    , $out
}
# 把图标写进图集，返回加了新子图的布局文本
function Add-StatusSprite([byte[]]$Atlas, [string]$Text, [string]$Name, [byte[]]$Icon) {
    if ($Text.Contains("name=""$Name""")) { throw "$Name already exists in $StatusAtlas" }
    $width = [BitConverter]::ToInt32($Atlas, 16); $height = [BitConverter]::ToInt32($Atlas, 12)
    $spot = Find-StatusSpot $Text $width $height
    $blocks = $StatusIconPx / 4
    for ($r = 0; $r -lt $blocks; $r++) {
        [Array]::Copy($Icon, $r * $blocks * $Bc7BlockBytes, $Atlas, $Bc7Header + (($spot.Y / 4 + $r) * ($width / 4) + $spot.X / 4) * $Bc7BlockBytes, $blocks * $Bc7BlockBytes)
    }
    $line = "<SubTexture name=""$Name"" x=""$($spot.X)"" width=""$StatusIconPx"" y=""$($spot.Y)"" height=""$StatusIconPx"" originalWidth=""$StatusIconPx"" originalHeight=""$StatusIconPx"" half=""0""/>"
    $nl = if ($Text.Contains("`r`n")) { "`r`n" } else { "`n" }
    $close = $Text.LastIndexOf("</TextureAtlas>")
    $Text.Substring(0, $close) + "`t" + $line + $nl + $Text.Substring($close)
}
function Get-ZeroLayerIcon([byte[]]$OneLayer) {
    $tmp = [IO.Path]::GetTempFileName()
    $in = "$tmp.in.bin"; $out = "$tmp.out.bin"
    try {
        [IO.File]::WriteAllBytes($in, $OneLayer)
        & python -I (Join-Path $PSScriptRoot "ga_icon_zero.py") $in $out | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "ga_icon_zero failed (needs Pillow + numpy)" }
        , [IO.File]::ReadAllBytes($out)
    } finally { Remove-Item $tmp, $in, $out -ErrorAction SilentlyContinue }
}
function Add-StatusIcons($GaTpf, $GaLay, $Tpf, $Lay) {
    $layFile = $Lay.Files | Where-Object { [IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $StatusAtlas } | Select-Object -First 1
    $dstTex = $Tpf.Textures | Where-Object { $_.Name -eq $StatusAtlas } | Select-Object -First 1
    [byte[]]$dst = $dstTex.Bytes
    if ((Get-DdsHeaderSize $dst) -ne $Bc7Header) { throw "$StatusAtlas is not BC7" }
    $text = [Text.Encoding]::UTF8.GetString($layFile.Bytes.ToArray())
    $sources = @{}
    foreach ($f in $GaLay.Files) { $sources[[IO.Path]::GetFileNameWithoutExtension($f.Name)] = [Text.Encoding]::UTF8.GetString($f.Bytes.ToArray()) }
    $oneLayer = $null
    foreach ($id in $StatusIconIds) {
        $name = "MENU_StatusIcon_$id.png"
        $atlas = $sources.Keys | Where-Object { $sources[$_].Contains("name=""$name""") } | Select-Object -First 1
        if (-not $atlas) { throw "$name not found in GA" }
        $s = (Get-Subs $sources[$atlas])[$name]
        if ($s.W -ne $StatusIconPx -or $s.X % 4 -or $s.Y % 4) { throw "$name`: unexpected rectangle" }
        [byte[]]$src = ($GaTpf.Textures | Where-Object { $_.Name -eq $atlas } | Select-Object -First 1).Bytes
        if ((Get-DdsHeaderSize $src) -ne $Bc7Header -or [BitConverter]::ToInt32($src, 128) -ne [BitConverter]::ToInt32($dst, 128)) { throw "$atlas format differs from $StatusAtlas" }
        [byte[]]$icon = Get-StatusIconBlocks $src $s
        if ($id -eq $VersatilityOneIcon) { $oneLayer = $icon }
        $text = Add-StatusSprite $dst $text $name $icon
    }
    if (-not $oneLayer) { throw "one layer icon $VersatilityOneIcon not in the list" }
    $text = Add-StatusSprite $dst $text "MENU_StatusIcon_$VersatilityZeroIcon.png" (Get-ZeroLayerIcon $oneLayer)
    $dstTex.Bytes = $dst
    $layFile.Bytes = [Text.Encoding]::UTF8.GetBytes($text)
    "状态图标：$($StatusIconIds.Count + 1) 个写进 $StatusAtlas"
}
