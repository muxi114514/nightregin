<#
  MMV 与 6 人修复包（Nightreign6PlayerFixes）的公共事件函数库 common_func.emevd 合并，修复卡莉果入侵等事件卡死。
  原因：6 人修复包把一批公共事件改成 6 人版（参数里多了 4~6 号玩家 / 帮手实体），并让自己的地图脚本（如卡莉果入侵 m53_05）按新参数调用；
  启动配置里 MMV 加载在它之后，MMV 的 common_func（仍是 3 人版）把它整份顶掉，6 人版地图脚本按 8 个参数调用 5 个参数的事件，
  参数整体错位（入侵区域实体读成了 4 号帮手），事件永远等不到条件，冰龙不出现、整局卡在事件中。
  做法：以 MMV 当前的 common_func 为底，把 6 人修复包相对 MMV 底本（.bak）改过的事件换成 6 人修复包的版本，但跳过：
    - MMV 自己改过的事件（与底本骨架不同）、6 人修复包没有的事件
    - 当前游戏版本没有的事件、6 人修复包参数更少的事件（两边游戏版本不同造成的差异，不是 6 人改动）
  写出后自检：6 人修复包与 MMV 的地图脚本调用公共事件时，参数个数都与各自原本按的版本一致。
  只能用在同时加载 6 人修复包的启动配置（只加载 MMV 时原版地图脚本按 3 人参数调用，反而会错位）。
#>
param(
    # MMV 的 event 目录（含 common_func.emevd.dcx 与底本 .bak）
    [Parameter(Mandatory)] [string]$MmvEvent,
    [Parameter(Mandatory)] [string]$SixPlayerEvent,
    # 当前游戏版本的 common_func（tools\extract_vanilla.ps1 导出）
    [Parameter(Mandatory)] [string]$VanillaCommonFunc,
    [Parameter(Mandatory)] [string]$OutDir,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
[AppDomain]::CurrentDomain.add_AssemblyResolve({ param($s, $e) $p = [IO.Path]::Combine($app, $e.Name.Split(',')[0] + '.dll'); if ([IO.File]::Exists($p)) { [Reflection.Assembly]::LoadFrom($p) } else { $null } }) | Out-Null
[Reflection.Assembly]::LoadFrom("$app\Andre.SoulsFormats.dll") | Out-Null

# DCX 的 KRAK 压缩要从当前目录加载 oo2core
function Invoke-InDeps([scriptblock]$Block) {
    $old = [IO.Directory]::GetCurrentDirectory()
    [IO.Directory]::SetCurrentDirectory($deps)
    try { & $Block } finally { [IO.Directory]::SetCurrentDirectory($old) }
}
function Read-Emevd([string]$Path) { $p = (Resolve-Path $Path).Path; Invoke-InDeps { [SoulsFormats.EMEVD]::Read($p) } }
function Get-Events($Emevd) { $h = @{}; foreach ($e in $Emevd.Events) { $h[[long]$e.ID] = $e }; $h }
# 指令骨架（只比 bank / id 序列）：MMV 用 DarkScript 重新编译过，条件组编号会变，按字节比较不可靠
function Get-Shape($Event) { if (-not $Event) { return "" }; ($Event.Instructions | ForEach-Object { "$($_.Bank).$($_.ID)" }) -join ' ' }
# 事件读取的参数个数（参数替换表里最远的源偏移）
function Get-ParamCount($Event) {
    if (-not $Event) { return $null }
    $max = 0
    foreach ($p in $Event.Parameters) { $max = [Math]::Max($max, $p.SourceStartByte + $p.ByteCount) }
    [int]($max / 4)
}
# 地图脚本里 InitializeCommonEvent（2000[06]）调用的 (事件 ID, 文件名)
function Get-CommonCalls([string]$Dir) {
    foreach ($f in Get-ChildItem $Dir -Filter "*.emevd.dcx" | Where-Object { $_.Name -ne "common_func.emevd.dcx" }) {
        foreach ($e in (Read-Emevd $f.FullName).Events) {
            foreach ($i in $e.Instructions) {
                if ($i.Bank -eq 2000 -and $i.ID -eq 6) { [pscustomobject]@{ File = $f.Name; Id = [long][BitConverter]::ToUInt32($i.ArgData, 4) } }
            }
        }
    }
}

$mmvFile = Read-Emevd "$MmvEvent\common_func.emevd.dcx"
$mmv = Get-Events $mmvFile
$base = Get-Events (Read-Emevd "$MmvEvent\common_func.emevd.dcx.bak")
$six = Get-Events (Read-Emevd "$SixPlayerEvent\common_func.emevd.dcx")
$vanilla = Get-Events (Read-Emevd $VanillaCommonFunc)

$take = @()
foreach ($id in ($six.Keys | Sort-Object)) {
    if ((Get-Shape $six[$id]) -eq (Get-Shape $base[$id])) { continue }
    if (-not $mmv.ContainsKey($id) -or (Get-Shape $mmv[$id]) -ne (Get-Shape $base[$id])) { continue }
    if (-not $vanilla.ContainsKey($id) -or (Get-ParamCount $six[$id]) -lt (Get-ParamCount $mmv[$id])) { continue }
    $take += $id
}
for ($i = 0; $i -lt $mmvFile.Events.Count; $i++) {
    $id = [long]$mmvFile.Events[$i].ID
    if ($take -contains $id) { $mmvFile.Events[$i] = $six[$id] }
}
$merged = Get-Events $mmvFile

# 自检：两边的地图脚本调用公共事件时，合并版的参数个数要等于它们原本按的版本
$problems = @()
foreach ($c in Get-CommonCalls $SixPlayerEvent) {
    if ($six.ContainsKey($c.Id) -and $merged.ContainsKey($c.Id) -and (Get-ParamCount $six[$c.Id]) -ne (Get-ParamCount $merged[$c.Id])) { $problems += "6p $($c.File) -> $($c.Id)" }
}
foreach ($c in Get-CommonCalls $MmvEvent) {
    if ($mmv.ContainsKey($c.Id) -and (Get-ParamCount $mmv[$c.Id]) -ne (Get-ParamCount $merged[$c.Id])) { $problems += "mmv $($c.File) -> $($c.Id)" }
}
if ($problems) { throw "common_func merge leaves mismatched calls: $(($problems | Sort-Object -Unique) -join ', ')" }

$out = Join-Path $OutDir "event\common_func.emevd.dcx"
New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null
Invoke-InDeps { $mmvFile.Write($out) }
"event\common_func.emevd.dcx: 换入 6 人修复包的事件 $($take.Count) 个（共 $($mmvFile.Events.Count) 个），参数自检通过"
