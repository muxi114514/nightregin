<#
  图鉴导出：从整合包读文本、参数和贴图，生成 <整合包>\launcher\data\wiki.json 和 img\*.webp（对游戏文件只读）。
  用法：powershell -ExecutionPolicy Bypass -File tools\wiki\export_wiki.ps1 -Version 1.9.5 [-Previous <上一版 regulation.bin>] [-PackRoot <整合包>] [-Work <工作目录>]
  -Previous 给了时，<工作目录>\notes_candidates.txt 列出比上一版新增的条目，供写 notes 时参考。
#>
param(
    [Parameter(Mandatory)] [string]$Version,
    [string]$PackRoot = '',
    [string]$Previous = '',
    [string]$Work = ''
)
$ErrorActionPreference = 'Stop'
if (-not $PackRoot) { $PackRoot = Join-Path $PSScriptRoot '..\..\..\me3\MMV\me3' }
if (-not $Work) { $Work = Join-Path $PSScriptRoot '..\..\target\wiki_work' }
New-Item -ItemType Directory -Force $Work | Out-Null
$Work = (Resolve-Path $Work).Path
$PackRoot = (Resolve-Path $PackRoot).Path

# 运行原生命令，只按退出码判成败
function Invoke-Step([string]$What, [scriptblock]$Command) {
    Write-Output "== $What"
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = $null
    & $Command 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { throw "$What 失败（退出码 $LASTEXITCODE）" }
}
function Invoke-Dump([string]$Script, [string[]]$More) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot $Script) @More
}

$out = Join-Path $Work 'out'
$menu = Join-Path $Work 'menu'
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Force $out | Out-Null
$build = Join-Path $PSScriptRoot 'build_wiki.py'
$sprites = Join-Path $PSScriptRoot 'menu_sprites.py'

Invoke-Step '导出文本' { Invoke-Dump 'dump_text.ps1' @('-Out', $Work) }
$paramArgs = @('-Out', $Work)
if ($Previous) { $paramArgs += @('-Previous', (Resolve-Path $Previous).Path) }
Invoke-Step '导出参数' { Invoke-Dump 'dump_params.ps1' $paramArgs }
Invoke-Step '导出贴图布局' { Invoke-Dump 'dump_menu.ps1' @('-Out', $menu) }
Invoke-Step '收集要用的子图' { python -I $build $Work $out $Version --sprites-only }
$atlases = (& python -I $sprites planfile $menu (Join-Path $Work 'sprites.txt') | Select-Object -Last 1).Trim()
if ($LASTEXITCODE -ne 0 -or -not $atlases) { throw '算不出要导出的图集' }
Invoke-Step '导出图集' { Invoke-Dump 'dump_menu.ps1' @('-Out', $menu, '-Atlases', $atlases) }
Invoke-Step '生成图鉴' { python -I $build $Work $out $Version }

$data = Join-Path $PackRoot 'launcher\data'
Invoke-Step '安装图鉴数据' { python -I (Join-Path $PSScriptRoot 'install_data.py') $out $data }
Write-Output "WIKI OK: $data"
