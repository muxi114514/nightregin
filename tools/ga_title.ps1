<#
  开场界面换成 GoldenAge 的黄金树标志 + 余烬背景（只给 MMV 系）。
    menu\01_common_h.tpf.dcx / .sblytbnd.dcx   追加图集 SB_GA_Title_0（余烬半尺寸 + 标志，DXT5，常驻约 38MB）
    menu\05_001_title_logo.gfx                  由原版开场文件改出（tools\ga_title.py）；MMV 只换过标题图，这个文件它不带
  就地改 <Pkg>\menu 里已有的贴图包（保留别的工具加进去的图）。重复运行会报错（图集已存在）。要 Python + Pillow + numpy
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$GaRoot = "D:\steam\steamapps\common\ELDEN RING\Game\GA",
    [string]$VanillaGfx = "$PSScriptRoot\..\target\vanilla\menu\05_001_title_logo.gfx",
    [string]$Game = "$PSScriptRoot\..\..\Game",
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

$Atlas = "SB_GA_Title_0"
$GaAtlases = "SB_Title_01", "SB_Title_02"
$Reference = "SB_Title_0"
$TitleGfx = "05_001_title_logo.gfx"

$pkgPath = (Resolve-Path $Pkg).Path
$tpfPath = "$pkgPath\menu\01_common_h.tpf.dcx"
$layPath = "$pkgPath\menu\01_common_h.sblytbnd.dcx"
if (-not (Test-Path $VanillaGfx)) {
    & (Join-Path $PSScriptRoot "extract_vanilla.ps1") -Game $Game -OutDir (Split-Path (Split-Path $VanillaGfx)) -Paths @("/menu/$TitleGfx") | Out-Null
}
$tmp = [IO.Path]::GetTempFileName()
$files = @{ T1 = "$tmp.t1.dds"; T2 = "$tmp.t2.dds"; Ref = "$tmp.ref.layout"; Dds = "$tmp.atlas.dds"; Layout = "$tmp.atlas.layout" }
try {
    # GA 的两张图集取出来交给 Python（只按块裁，不整张解码）。TPF 可枚举，返回时加逗号免得被拆成贴图数组
    $ga = Invoke-InDeps { , [SoulsFormats.TPF]::Read("$GaRoot\menu\hi\01_common.tpf.dcx") }
    foreach ($name in $GaAtlases) {
        $t = $ga.Textures | Where-Object { $_.Name -eq $name } | Select-Object -First 1
        if (-not $t) { throw "GA atlas $name not found" }
        [IO.File]::WriteAllBytes($(if ($name -eq $GaAtlases[0]) { $files.T1 } else { $files.T2 }), [byte[]]$t.Bytes)
    }
    $ga = $null; [GC]::Collect()

    $tpf = Invoke-InDeps { , [SoulsFormats.TPF]::Read($tpfPath) }
    $lay = Read-Bnd $layPath
    if ($tpf.Textures | Where-Object { $_.Name -eq $Atlas }) { throw "$Atlas already exists (already converted?)" }
    $refFile = $lay.Files | Where-Object { [IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $Reference } | Select-Object -First 1
    [IO.File]::WriteAllBytes($files.Ref, $refFile.Bytes.ToArray())
    & python -I (Join-Path $PSScriptRoot "ga_title.py") atlas $files.T1 $files.T2 $files.Ref $files.Dds $files.Layout
    if ($LASTEXITCODE -ne 0) { throw "ga_title.py atlas failed" }

    $template = $tpf.Textures | Where-Object { $_.Name -eq $Reference } | Select-Object -First 1
    $tex = New-Object SoulsFormats.TPF+Texture($Atlas, [byte]$template.Format, [byte]$template.Flags1, [IO.File]::ReadAllBytes($files.Dds), $template.Platform)
    $tpf.Textures.Add($tex)
    $layout = New-Object SoulsFormats.BinderFile
    $layout.Flags = $refFile.Flags
    $layout.CompressionType = $refFile.CompressionType
    $layout.ID = ($lay.Files | ForEach-Object { $_.ID } | Measure-Object -Maximum).Maximum + 1
    $layout.Name = [IO.Path]::GetDirectoryName($refFile.Name) + "\$Atlas.layout"
    $layout.Bytes = [IO.File]::ReadAllBytes($files.Layout)
    $lay.Files.Add($layout)
    Invoke-InDeps { $tpf.Write($tpfPath) }
    Write-Bnd $lay $layPath
    "01_common_h：加图集 $Atlas（$([Math]::Round($tex.Bytes.Length / 1MB, 1)) MB），布局 $($layout.ID)"
} finally { Remove-Item $tmp, $files.Values -ErrorAction SilentlyContinue }

& python -I (Join-Path $PSScriptRoot "ga_title.py") gfx (Resolve-Path $VanillaGfx).Path "$GaRoot\menu\$TitleGfx" "$pkgPath\menu\$TitleGfx"
if ($LASTEXITCODE -ne 0) { throw "ga_title.py gfx failed" }

# 自检：读回图集和布局
$check = Invoke-InDeps { , [SoulsFormats.TPF]::Read($tpfPath) }
if (-not ($check.Textures | Where-Object { $_.Name -eq $Atlas })) { throw "self-check: $Atlas missing" }
$checkLay = Read-Bnd $layPath
if (-not ($checkLay.Files | Where-Object { $_.Name.EndsWith("\$Atlas.layout") })) { throw "self-check: layout missing" }
"ga_title 完成"
