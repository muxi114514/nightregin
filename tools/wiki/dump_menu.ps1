<#
  图鉴导出第三步：从菜单贴图包导出图集布局与指定图集的 DDS（只读）。
  不带 -Atlases：导出每个包的全部布局到 <Out>\<包标签>\layouts\，并在 <Out>\<包标签>\textures.txt 列出贴图名与大小。
  带 -Atlases：再把这些图集的 DDS 导出到 <Out>\<包标签>\dds\（同名图集按 $Menus 顺序取第一个有的包）。
#>
param(
    [string[]]$Menus = @(
        "pkg|D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\MMV\me3\StatAlloc\pkg_mmv\menu\01_common_h",
        "mmv|D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\MMV\me3\mod\menu\01_common_h",
        "mmv_dlc|D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\MMV\me3\mod\menu\21_common_dlc01_h",
        "van|D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\NRStatAlloc\target\vanilla\menu\01_common_h",
        "van_dlc|D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\NRStatAlloc\target\vanilla\menu\21_common_dlc01_h"
    ),
    [string[]]$Atlases = @(),
    [string]$Erbm = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\NightreignPLUS\ERBMv1.1\Files",
    [Parameter(Mandatory)] [string]$Out
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\NRStatAlloc\tools\tae_helpers.ps1"

# powershell -File 传数组参数时会合成一个 "a,b,c" 字符串，这里拆开
if ($Atlases.Count -eq 1 -and $Atlases[0] -match ',') { $Atlases = $Atlases[0] -split ',' }
$pending = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($a in $Atlases) { [void]$pending.Add($a) }
foreach ($m in $Menus) {
    $tag, $base = $m -split '\|', 2
    $dir = Join-Path $Out $tag
    if (-not $Atlases.Count) {
        if (-not (Test-Path "$base.sblytbnd.dcx")) { Write-Warning "skip $base"; continue }
        New-Item -ItemType Directory -Force "$dir\layouts" | Out-Null
        $lay = Read-Bnd "$base.sblytbnd.dcx"
        foreach ($f in $lay.Files) { [IO.File]::WriteAllBytes((Join-Path "$dir\layouts" ([IO.Path]::GetFileName($f.Name))), $f.Bytes.ToArray()) }
        $tpf = Invoke-InDeps { , [SoulsFormats.TPF]::Read("$base.tpf.dcx") }
        ($tpf.Textures | ForEach-Object { "{0}`t{1}" -f $_.Name, $_.Bytes.Length }) | Set-Content -Encoding UTF8 "$dir\textures.txt"
        "{0}: {1} layouts, {2} textures" -f $tag, $lay.Files.Count, $tpf.Textures.Count
        $tpf = $null; [GC]::Collect()
        continue
    }
    if ($pending.Count -eq 0) { break }
    if (-not (Test-Path "$base.tpf.dcx")) { continue }
    $tpf = Invoke-InDeps { , [SoulsFormats.TPF]::Read("$base.tpf.dcx") }
    New-Item -ItemType Directory -Force "$dir\dds" | Out-Null
    foreach ($t in $tpf.Textures) {
        if (-not $pending.Contains($t.Name)) { continue }
        [IO.File]::WriteAllBytes((Join-Path "$dir\dds" "$($t.Name).dds"), [byte[]]$t.Bytes)
        [void]$pending.Remove($t.Name)
        "{0}: {1}" -f $tag, $t.Name
    }
    $tpf = $null; [GC]::Collect()
}
if ($Atlases.Count -and $pending.Count) { Write-Warning ("not found: " + ($pending -join ", ")) }
