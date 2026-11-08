<#
  MMV 把 DLC 的湖沼 / 锻造村据点放进了普通宁姆韦德，地图上不显示图标：普通地图的据点图标要两张图都在基础图集 01_common_h 里——
  地图界面 02_120_worldmap.gfx 按图标行号跳帧显示的 MENU_MAP_<行>，以及按图标行的 iconId 取的 MENU_MenuIcon_<iconId>；
  DLC 那两行（77 / 78）的图只在 DLC 图集 21_common_dlc01_h 里。照 MMV 加自己图标行 122 / 126 的做法，据点改用图标行 124 / 125、
  菜单图标号 42632 / 42633（regulation 部分见 relics\src\content\map_icons.rs，mmv_relics --mmv-map-icons），这里把两种图从 DLC 图集搬进来：
    - MENU_MAP_124 / 125：搬进 SB_MapCursor_02_0 的空白处，把已有的同名占位条目改指向它
    - MENU_MenuIcon_42632 / 42633：搬进 SB_MenuIcon_1 的空白处（紧挨 MMV 的 42630 / 42631），新增条目
  按 BC7 4x4 块原样复制（两边都是 BC7、1 级 mip，不重新压缩），目标位置 4 对齐、不压任何已有子图。
  输出 <OutDir>\menu\01_common_h.tpf.dcx 与 01_common_h.sblytbnd.dcx，只给 MMV 系 pkg 用（tools\build_mmv_pkg.ps1 -Target Mmv）。
#>
param(
    # 其下为 01_common_h / 21_common_dlc01_h 的 .tpf.dcx 与 .sblytbnd.dcx（MMV 的 mod\menu）
    [Parameter(Mandatory)] [string]$MenuDir,
    [Parameter(Mandatory)] [string]$OutDir,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
[AppDomain]::CurrentDomain.add_AssemblyResolve({ param($s, $e) $p = [IO.Path]::Combine($app, $e.Name.Split(',')[0] + '.dll'); if ([IO.File]::Exists($p)) { [Reflection.Assembly]::LoadFrom($p) } else { $null } }) | Out-Null
[Reflection.Assembly]::LoadFrom("$app\Andre.SoulsFormats.dll") | Out-Null

# DDS 头 128 + DX10 扩展头 20
$DdsHeader = 148
$BlockBytes = 16
# Repoint：改已有同名条目的坐标；否则新增条目。位置是在 MMV 的 01_common_h 里找好的空白处（像素全透明）
$Copies = @(
    @{ SrcAtlas = "SB_MapIcon_DLC01_0"; From = "MENU_MAP_42047_077.png"; DstAtlas = "SB_MapCursor_02_0"; Name = "MENU_MAP_124.png"; X = 2104; Y = 4; Repoint = $true },
    @{ SrcAtlas = "SB_MapIcon_DLC01_0"; From = "MENU_MAP_42048_078.png"; DstAtlas = "SB_MapCursor_02_0"; Name = "MENU_MAP_125.png"; X = 2380; Y = 4; Repoint = $true },
    @{ SrcAtlas = "SB_MenuIcon_DLC01_0"; From = "MENU_MenuIcon_42047.png"; DstAtlas = "SB_MenuIcon_1"; Name = "MENU_MenuIcon_42632.png"; X = 3104; Y = 932; Repoint = $false },
    @{ SrcAtlas = "SB_MenuIcon_DLC01_0"; From = "MENU_MenuIcon_42048.png"; DstAtlas = "SB_MenuIcon_1"; Name = "MENU_MenuIcon_42633.png"; X = 3312; Y = 932; Repoint = $false }
)

function Invoke-InDeps([scriptblock]$Block) {
    $old = [IO.Directory]::GetCurrentDirectory()
    [IO.Directory]::SetCurrentDirectory($deps)
    try { & $Block } finally { [IO.Directory]::SetCurrentDirectory($old) }
}
function Get-Layout($Bnd, [string]$Atlas) {
    $f = $Bnd.Files | Where-Object { [IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $Atlas } | Select-Object -First 1
    if (-not $f) { throw "layout $Atlas not found" }
    $f
}
function Get-Texture($Tpf, [string]$Atlas) {
    $t = $Tpf.Textures | Where-Object { $_.Name -eq $Atlas } | Select-Object -First 1
    if (-not $t) { throw "texture $Atlas not found" }
    $t
}
# 布局里的子图：名字 -> @{X;Y;W;H;Line}
function Get-SubTextures([string]$Text) {
    $map = @{}
    foreach ($m in [regex]::Matches($Text, '<SubTexture\s[^>]*/>')) {
        $a = @{}; foreach ($kv in [regex]::Matches($m.Value, '(\w+)="([^"]*)"')) { $a[$kv.Groups[1].Value] = $kv.Groups[2].Value }
        $map[$a.name] = @{ X = [int]$a.x; Y = [int]$a.y; W = [int]$a.width; H = [int]$a.height; Line = $m.Value }
    }
    $map
}
function Test-Overlap($A, $B) { $A.X -lt $B.X + $B.W -and $B.X -lt $A.X + $A.W -and $A.Y -lt $B.Y + $B.H -and $B.Y -lt $A.Y + $A.H }
function Set-Attr([string]$Line, [string]$Name, $Value) { [regex]::Replace($Line, "\b$Name=""[^""]*""", "$Name=""$Value""") }

$menu = (Resolve-Path $MenuDir).Path
Invoke-InDeps {
    $baseTpf = [SoulsFormats.TPF]::Read("$menu\01_common_h.tpf.dcx")
    $baseLay = [SoulsFormats.BND4]::Read("$menu\01_common_h.sblytbnd.dcx")
    $dlcTpf = [SoulsFormats.TPF]::Read("$menu\21_common_dlc01_h.tpf.dcx")
    $dlcLay = [SoulsFormats.BND4]::Read("$menu\21_common_dlc01_h.sblytbnd.dcx")
    # 目标图集：贴图字节与布局文本，全部改完再写回
    $dst = @{}
    foreach ($atlas in ($Copies | ForEach-Object { $_.DstAtlas } | Sort-Object -Unique)) {
        $file = Get-Layout $baseLay $atlas
        $dst[$atlas] = @{ Tex = Get-Texture $baseTpf $atlas; Bytes = [byte[]](Get-Texture $baseTpf $atlas).Bytes; Layout = $file
            Text = [Text.Encoding]::UTF8.GetString($file.Bytes.ToArray()) }
    }
    foreach ($c in $Copies) {
        [byte[]]$src = (Get-Texture $dlcTpf $c.SrcAtlas).Bytes
        $s = (Get-SubTextures ([Text.Encoding]::UTF8.GetString((Get-Layout $dlcLay $c.SrcAtlas).Bytes.ToArray())))[$c.From]
        if (-not $s) { throw "$($c.From) not found in $($c.SrcAtlas)" }
        $d = $dst[$c.DstAtlas]
        $existing = Get-SubTextures $d.Text
        $old = $existing[$c.Name]
        if ($c.Repoint -and -not $old) { throw "$($c.Name) not found in $($c.DstAtlas)" }
        if ($old -and ($old.X -eq $c.X -and $old.Y -eq $c.Y -or -not $c.Repoint)) { throw "$($c.Name) already in $($c.DstAtlas) (input is not MMV's original 01_common_h?)" }
        $target = @{ X = $c.X; Y = $c.Y; W = $s.W; H = $s.H }
        $hit = $existing.Keys | Where-Object { $_ -ne $c.Name -and (Test-Overlap $existing[$_] $target) } | Select-Object -First 1
        if ($hit) { throw "$($c.Name) at ($($c.X),$($c.Y)) overlaps $hit" }
        if ($s.X % 4 -or $s.Y % 4 -or $c.X % 4 -or $c.Y % 4) { throw "$($c.From): not block aligned" }
        # DDS 宽度在 +16，BC7 每行块数 = 宽 / 4
        $srcRow = [BitConverter]::ToInt32($src, 16) / 4; $dstRow = [BitConverter]::ToInt32($d.Bytes, 16) / 4
        $blocksW = [Math]::Ceiling($s.W / 4); $blocksH = [Math]::Ceiling($s.H / 4)
        for ($r = 0; $r -lt $blocksH; $r++) {
            $from = $DdsHeader + (($s.Y / 4 + $r) * $srcRow + $s.X / 4) * $BlockBytes
            $to = $DdsHeader + (($c.Y / 4 + $r) * $dstRow + $c.X / 4) * $BlockBytes
            [Array]::Copy($src, $from, $d.Bytes, $to, $blocksW * $BlockBytes)
        }
        $line = if ($old) { $old.Line } else { $s.Line.Replace("name=""$($c.From)""", "name=""$($c.Name)""") }
        foreach ($kv in @(@("x", $c.X), @("y", $c.Y), @("width", $s.W), @("height", $s.H), @("originalWidth", $s.W), @("originalHeight", $s.H))) { $line = Set-Attr $line $kv[0] $kv[1] }
        if ($old) {
            $d.Text = $d.Text.Replace($old.Line, $line)
        } else {
            $nl = if ($d.Text.Contains("`r`n")) { "`r`n" } else { "`n" }
            $close = $d.Text.LastIndexOf("</TextureAtlas>")
            $d.Text = $d.Text.Substring(0, $close) + "`t" + $line + $nl + $d.Text.Substring($close)
        }
    }
    foreach ($d in $dst.Values) {
        $d.Tex.Bytes = $d.Bytes
        $d.Layout.Bytes = [Text.Encoding]::UTF8.GetBytes($d.Text)
    }
    $out = Join-Path $OutDir "menu"
    New-Item -ItemType Directory -Force $out | Out-Null
    $baseTpf.Write("$out\01_common_h.tpf.dcx")
    $baseLay.Write("$out\01_common_h.sblytbnd.dcx")
}
"menu\01_common_h: " + (($Copies | ForEach-Object { "$($_.Name -replace '\.png$') <- $($_.From -replace '\.png$')" }) -join '，')
