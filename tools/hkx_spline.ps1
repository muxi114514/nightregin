<#
  读 hkx（TAG0，SDK 20180100）里的样条压缩动画：找到某根骨骼的位移轨道在文件里的位置，方便就地改量化范围（文件大小不变）。
  布局参考 hkaSplineCompressedAnimation；块内每条轨道依次是位移、旋转、缩放，各自 4 字节对齐（相对动画数据开头）。
#>

# 动画对象的字段
$HkxTrackCount = 0x20
$HkxFrameInfo = 0x40
$HkxBlockOffsets = 0x60
$HkxData = 0xA0
$HkxRotationSize = @{ 0 = 4; 1 = 5; 2 = 6; 3 = 3; 4 = 2; 5 = 16 }
$HkxRotationAlign = @{ 0 = 4; 1 = 1; 2 = 2; 3 = 1; 4 = 2; 5 = 4 }

function Get-HkxSections([byte[]]$B, [int]$Start, [int]$End) {
    $pos = $Start
    while ($pos + 8 -le $End) {
        $size = (([int]$B[$pos] -band 0x3F) -shl 24) -bor ([int]$B[$pos + 1] -shl 16) -bor ([int]$B[$pos + 2] -shl 8) -bor [int]$B[$pos + 3]
        @{ Tag = [Text.Encoding]::ASCII.GetString($B, $pos + 4, 4); Start = $pos + 8; End = $pos + $size }
        $pos += $size
    }
}
function Get-HkxSection($Sections, [string]$Tag) {
    $s = $Sections | Where-Object { $_.Tag -eq $Tag } | Select-Object -First 1
    if (-not $s) { throw "hkx section $Tag not found" }
    $s
}
function Get-HkxAligned([int]$P, [int]$N, [int]$Base) {
    $r = ($P - $Base) % $N
    if ($r -eq 0) { $P } else { $P + $N - $r }
}
# 跳过一段位移 / 缩放；$Channels 不为空时记下各分量（0 / 1 / 2 = X / Y / Z）的类型、文件位置与范围
function Skip-HkxVector([byte[]]$B, [int]$P, [int]$Flags, [int]$Quant, [int]$Blob, $Channels) {
    $splines = 0
    $count = 0
    if ($Flags -band 0x70) {
        $count = [BitConverter]::ToUInt16($B, $P)
        $P = Get-HkxAligned ($P + 3 + $count + $B[$P + 2] + 2) 4 $Blob
    }
    for ($c = 0; $c -lt 3; $c++) {
        if ($Flags -band (0x10 -shl $c)) {
            if ($null -ne $Channels) { $Channels[$c] = @{ Spline = $true; Offset = $P; Lo = [BitConverter]::ToSingle($B, $P); Hi = [BitConverter]::ToSingle($B, $P + 4) } }
            $P += 8
            $splines++
        } elseif ($Flags -band (1 -shl $c)) {
            if ($null -ne $Channels) { $Channels[$c] = @{ Spline = $false; Offset = $P; Lo = [BitConverter]::ToSingle($B, $P); Hi = [BitConverter]::ToSingle($B, $P) } }
            $P += 4
        }
    }
    if ($splines) { $P += ($count + 1) * $splines * $(if ($Quant -eq 0) { 1 } else { 2 }) }
    Get-HkxAligned $P 4 $Blob
}
function Skip-HkxRotation([byte[]]$B, [int]$P, [int]$Flags, [int]$Quant, [int]$Blob) {
    if ($Flags -band 0xF0) {
        $count = [BitConverter]::ToUInt16($B, $P)
        $P = Get-HkxAligned ($P + 3 + $count + $B[$P + 2] + 2) $HkxRotationAlign[$Quant] $Blob
        $P += ($count + 1) * $HkxRotationSize[$Quant]
    } elseif ($Flags -band 0x0F) {
        $P = Get-HkxAligned $P $HkxRotationAlign[$Quant] $Blob
        $P += $HkxRotationSize[$Quant]
    }
    Get-HkxAligned $P 4 $Blob
}
# 返回每一块里该骨骼位移的分量表；走完一块校验落在下一块开头之前，解析错位时直接报错
function Get-HkxBonePosition([byte[]]$B, [int]$Bone) {
    $tag0 = Get-HkxSection @(Get-HkxSections $B 0 $B.Length) "TAG0"
    $inner = @(Get-HkxSections $B $tag0.Start $tag0.End)
    $data = Get-HkxSection $inner "DATA"
    $index = Get-HkxSection $inner "INDX"
    $itemSec = Get-HkxSection @(Get-HkxSections $B $index.Start $index.End) "ITEM"
    $items = @()
    for ($at = $itemSec.Start; $at + 12 -le $itemSec.End; $at += 12) {
        $items += @{ Off = [BitConverter]::ToInt32($B, $at + 4); Count = [BitConverter]::ToInt32($B, $at + 8) }
    }
    $offsets = @($items | ForEach-Object { $_.Off } | Sort-Object -Unique)
    $next = @{}
    for ($i = 0; $i -lt $offsets.Count; $i++) { $next[$offsets[$i]] = $(if ($i + 1 -lt $offsets.Count) { $offsets[$i + 1] } else { $data.End - $data.Start }) }

    # 动画数据是最大的字节数组，动画对象的数据字段指向它
    $blobIndex = -1
    $blobSize = 0
    for ($i = 0; $i -lt $items.Count; $i++) {
        $size = $next[$items[$i].Off] - $items[$i].Off
        if ($items[$i].Count -gt 0 -and [Math]::Abs($size - $items[$i].Count) -lt 16 -and $size -gt $blobSize) { $blobIndex = $i; $blobSize = $size }
    }
    $anim = $null
    foreach ($item in $items) {
        if ($item.Count -eq 1 -and $item.Off + $HkxData + 8 -le $data.End - $data.Start -and [BitConverter]::ToInt32($B, $data.Start + $item.Off + $HkxData) -eq $blobIndex) { $anim = $data.Start + $item.Off }
    }
    if (-not $anim) { throw "spline animation not found" }
    $tracks = [BitConverter]::ToInt32($B, $anim + $HkxTrackCount)
    $blockCount = [BitConverter]::ToInt32($B, $anim + $HkxFrameInfo + 4)
    $maskSize = [BitConverter]::ToInt32($B, $anim + $HkxFrameInfo + 12)
    $blockItem = $items[[BitConverter]::ToInt32($B, $anim + $HkxBlockOffsets)]

    # 轨道 → 骨骼：长度等于轨道数的 s16 数组
    $track = -1
    foreach ($item in $items) {
        if ($item.Count -ne $tracks -or [Math]::Floor(($next[$item.Off] - $item.Off) / $tracks) -ne 2) { continue }
        for ($t = 0; $t -lt $tracks; $t++) {
            if ([BitConverter]::ToInt16($B, $data.Start + $item.Off + $t * 2) -eq $Bone) { $track = $t }
        }
    }
    if ($track -lt 0) { throw "bone $Bone has no track" }

    $blob = $data.Start + $items[$blobIndex].Off
    for ($block = 0; $block -lt $blockCount; $block++) {
        $blockOffset = [BitConverter]::ToInt32($B, $data.Start + $blockItem.Off + $block * 4)
        $end = $(if ($block + 1 -lt $blockCount) { [BitConverter]::ToInt32($B, $data.Start + $blockItem.Off + ($block + 1) * 4) } else { $blobSize }) + $blob
        $masks = $blob + $blockOffset
        $cursor = Get-HkxAligned ($masks + $maskSize) 4 $blob
        $channels = @{}
        for ($t = 0; $t -lt $tracks; $t++) {
            $quant = [int]$B[$masks + $t * 4]
            $posFlags = [int]$B[$masks + $t * 4 + 1]
            $rotFlags = [int]$B[$masks + $t * 4 + 2]
            $scaleFlags = [int]$B[$masks + $t * 4 + 3]
            $target = $null
            if ($t -eq $track) { $target = $channels }
            $cursor = Skip-HkxVector $B $cursor $posFlags ($quant -band 3) $blob $target
            $cursor = Skip-HkxRotation $B $cursor $rotFlags (($quant -shr 2) -band 15) $blob
            $cursor = Skip-HkxVector $B $cursor $scaleFlags (($quant -shr 6) -band 3) $blob $null
        }
        if ($cursor -gt $end -or $end - $cursor -ge 16) { throw "spline block $block parsed to $cursor, expected near $end" }
        @{ Block = $block; Channels = $channels }
    }
}
function Set-HkxSplineLow([byte[]]$B, $Channel, [single]$Low) {
    if (-not $Channel.Spline) { throw "channel is not a spline" }
    [BitConverter]::GetBytes($Low).CopyTo($B, $Channel.Offset)
}
