<#
  从本机法环加密归档取出 GoldenAge 特效用到、而 GA 包和黑夜君临都没有的资源，放到 target\ga_er_fx（ga_sfx.ps1 从这里读）。
  这些资源都在法环的 mimic 特效包里，$Banks 写明了编号在哪个包；每个编号的贴图、模型、动画都取出来，用哪些由 ga_sfx.ps1 决定。
  归档的打开方式见 fs_archive.ps1 的 Open-ErArchives。
#>
param(
    [string]$ErGame = "D:\steam\steamapps\common\ELDEN RING\Game",
    [string]$Smithbox = "D:\relink\114514\smithbox",
    [string]$OutDir = "$PSScriptRoot\..\target\ga_er_fx",
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")
. (Join-Path $PSScriptRoot "fs_archive.ps1")

$Banks = @{
    "sfxbnd_mimic000302615" = 89280; "sfxbnd_mimic000302627" = 89400; "sfxbnd_mimic000302631" = 89440; "sfxbnd_mimic000302635" = 89480
    "sfxbnd_mimic000302647" = 89600; "sfxbnd_mimic000302648" = 89610; "sfxbnd_mimic000302655" = 89680; "sfxbnd_mimic000302659" = 89720
}
New-Item -ItemType Directory -Force $OutDir | Out-Null
$outRoot = (Resolve-Path $OutDir).Path
$er = Open-ErArchives $ErGame $Smithbox $outRoot
$total = 0
foreach ($bankName in $Banks.Keys) {
    $number = $Banks[$bankName]
    [byte[]]$bytes = Get-ErFile $er "/sfx/$bankName.ffxbnd.dcx"
    if (-not $bytes) { throw "$bankName not found in the Elden Ring archives" }
    $bnd = Invoke-InDeps { [SoulsFormats.BND4]::Read([byte[]]$bytes) }
    $pattern = "^s0*$number(_[a-z0-9]+)?\.(tpf|flver|anibnd)$"
    $n = 0
    foreach ($f in $bnd.Files) {
        $leaf = [IO.Path]::GetFileName($f.Name.ToLower())
        if ($leaf -match $pattern) { [IO.File]::WriteAllBytes((Join-Path $outRoot $leaf), $f.Bytes.ToArray()); $n++; $total++ }
    }
    if ($n -eq 0) { throw "$bankName has no resource numbered $number" }
    "  $bankName → s$number：$n 个文件"
}
"取出 $total 个文件到 $outRoot"
