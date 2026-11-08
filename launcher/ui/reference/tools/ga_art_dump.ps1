# Read-only: dump selected GA menu atlases (raw DDS) and all 01_common layouts for launcher mockups
param(
    [string]$GaMenu = "D:\steam\steamapps\common\ELDEN RING\Game\GA\menu",
    [string]$Tools = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\NRStatAlloc\tools",
    [string]$Erbm = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\NightreignPLUS\ERBMv1.1\Files",
    [Parameter(Mandatory)] [string]$Out
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $Tools "tae_helpers.ps1")

$want = @("SB_Title", "SB_Title_01", "SB_Title_02", "SB_MainMenu", "SB_MainMenu_02", "SB_MainMenu_03",
          "SB_Tab", "SB_Tab_02", "SB_GA_MainMenu", "MENU_Gradation", "MENU_KUROOBI",
          "SB_KeyTop", "SB_KeyTop_02")
New-Item -ItemType Directory -Force "$Out\dds", "$Out\layouts" | Out-Null
$tpf = Invoke-InDeps { , [SoulsFormats.TPF]::Read("$GaMenu\hi\01_common.tpf.dcx") }
foreach ($t in $tpf.Textures) {
    if ($want -contains $t.Name) { [IO.File]::WriteAllBytes("$Out\dds\$($t.Name).dds", [byte[]]$t.Bytes); "dds $($t.Name)" }
}
$tpf = $null; [GC]::Collect()
$lay = Read-Bnd "$GaMenu\hi\01_common.sblytbnd.dcx"
foreach ($file in $lay.Files) {
    [IO.File]::WriteAllBytes("$Out\layouts\$([IO.Path]::GetFileName($file.Name))", $file.Bytes.ToArray())
}
"layouts $($lay.Files.Count)"
