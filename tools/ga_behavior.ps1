<#
  给行为图加 GoldenAge 招式表的片段（跟在 ga_moveset.ps1 之后运行）：
  片段清单取 GA 行为图里匹配 -Pattern 的动画名，由 tools\hkb_add_clips.py 克隆兄弟片段挂到黑夜君临的选择器上，
  结果写到 <Pkg>\chr\c0000.behbnd.dcx（-BehbndSrc 取整合包的行为图，不改原文件）。要 Python
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$BehbndSrc = "$PSScriptRoot\..\..\me3\MMV\me3\mod\chr\c0000.behbnd.dcx",
    [string]$GaRoot = "D:\steam\steamapps\common\ELDEN RING\Game\GA",
    [string]$Pattern = "a(073|373)_\d{6}",
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

$Graph = "\behaviors\c0000.hkx"
$pkgPath = (Resolve-Path $Pkg).Path
$latin = [Text.Encoding]::GetEncoding(28591)
$ga = Read-Bnd "$GaRoot\chr\c0000.behbnd.dcx"
$text = $latin.GetString((Find-File $ga $Graph).Bytes.ToArray())
$names = @([regex]::Matches($text, "\b$Pattern\b") | ForEach-Object { $_.Value } | Sort-Object -Unique)
if (-not $names.Count) { throw "no clips match $Pattern in GA" }

$bnd = Read-Bnd (Resolve-Path $BehbndSrc).Path
$file = Find-File $bnd $Graph
$tmp = [IO.Path]::GetTempFileName()
$in = "$tmp.in.hkx"; $out = "$tmp.out.hkx"; $list = "$tmp.txt"
try {
    [IO.File]::WriteAllBytes($in, $file.Bytes.ToArray())
    [IO.File]::WriteAllLines($list, [string[]]$names)
    & python -I (Join-Path $PSScriptRoot "hkb_add_clips.py") $in $out $list
    if ($LASTEXITCODE -ne 0) { throw "hkb_add_clips failed" }
    $file.Bytes = [IO.File]::ReadAllBytes($out)
} finally { Remove-Item $tmp, $in, $out, $list -ErrorAction SilentlyContinue }
Write-Bnd $bnd "$pkgPath\chr\c0000.behbnd.dcx"
"c0000.behbnd：片段清单 $($names.Count) 个"
