<#
  三档锻体的公共事件部分（只有 MMV，跟在 merge_6p_common_func.ps1 之后就地改 pkg 的 event\common_func.emevd.dcx）。
    紫16  开启监牢的事件 90045000：先查手里有没有石剑钥匙（3.4 + 跳过 1000.1），开门时扣 1 把（2003.24）。
          两处前面各插一对「本机玩家（10000）身上有锻体标记 9971670（relics\src\tiered\event.rs）就跳过」：
          4.5 IfCharacterHasSpEffect(条件组, 10000, 效果, 有, 等于, 1) + 1000.1 SkipIfConditionGroupStateUncompiled(行数, 通过, 条件组)
  按特征定位（6 人修复包合并后中间多了等待其他玩家的指令，不按整段序列核对）：查钥匙的 3.4 与扣钥匙的 2003.24 都要恰好一处，
  3.4 后面必须紧跟跳过指令；对不上直接报错；重复运行会报错（已改过）
#>
param(
    [Parameter(Mandatory)] [string]$Emevd,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
. (Join-Path $PSScriptRoot "tae_helpers.ps1")

$GaolEvent = 90045000
$Marker = 9971670
$LocalPlayer = 10000
$StoneswordKey = 8000

function New-Instruction([int]$Bank, [int]$Id, [byte[]]$ArgData) { New-Object SoulsFormats.EMEVD+Instruction($Bank, $Id, $ArgData) }
function Bytes([object[]]$Parts) {
    $ms = New-Object IO.MemoryStream
    foreach ($p in $Parts) { $b = if ($p -is [byte[]]) { $p } else { [BitConverter]::GetBytes($p) }; $ms.Write($b, 0, $b.Length) }
    , $ms.ToArray()
}
# 身上有标记时跳过接下来的 $Lines 条（条件组各用一个，不和原有的组 1 混用）
function New-SkipIfMarked([int]$Group, [int]$Lines) {
    $has = Bytes @([byte[]]@([byte]$Group, 0, 0, 0), [uint32]$LocalPlayer, [int]$Marker, [byte[]]@(1, 0, 0, 0), [float]1.0)
    @((New-Instruction 4 5 $has), (New-Instruction 1000 1 ([byte[]]@([byte]$Lines, 1, [byte]$Group, 0))))
}

$path = (Resolve-Path $Emevd).Path
$commonFunc = Invoke-InDeps { [SoulsFormats.EMEVD]::Read($path) }
$gaol = $commonFunc.Events | Where-Object { $_.ID -eq $GaolEvent } | Select-Object -First 1
if (-not $gaol) { throw "event $GaolEvent not found" }
$old = @($gaol.Instructions)
# 指令参数 +4 是物品 ID 的那条（3.4 查持有、2003.24 扣除）
function Find-KeyInstruction([int]$Bank, [int]$Id) {
    $hits = @(for ($i = 0; $i -lt $old.Count; $i++) {
        if ($old[$i].Bank -eq $Bank -and $old[$i].ID -eq $Id -and [BitConverter]::ToInt32($old[$i].ArgData, 4) -eq $StoneswordKey) { $i }
    })
    if ($hits.Count -ne 1) { throw "event ${GaolEvent}: expected 1 Stonesword Key instruction $Bank.$Id, found $($hits.Count)" }
    $hits[0]
}
if ($old | Where-Object { $_.Bank -eq 4 -and $_.ID -eq 5 -and [BitConverter]::ToInt32($_.ArgData, 8) -eq $Marker }) { throw "event ${GaolEvent}: already patched" }
$keyCheck = Find-KeyInstruction 3 4
$removeKey = Find-KeyInstruction 2003 24
$skip = $old[$keyCheck + 1]
if ($skip.Bank -ne 1000 -or $skip.ID -ne 1) { throw "event ${GaolEvent}: key check is not followed by a skip" }
# 查持有 + 跳过 + 被跳过的「提示、等待、重启」
$keyCheckLength = 2 + $skip.ArgData[0]

# 新序列与旧序号的对应（参数替换按指令序号，要跟着挪）
$new = New-Object Collections.Generic.List[object]
$map = @{}
for ($i = 0; $i -lt $old.Count; $i++) {
    if ($i -eq $keyCheck) { New-SkipIfMarked 2 $keyCheckLength | ForEach-Object { $new.Add($_) } }
    if ($i -eq $removeKey) { New-SkipIfMarked 3 1 | ForEach-Object { $new.Add($_) } }
    $map[$i] = $new.Count
    $new.Add($old[$i])
}
$gaol.Instructions.Clear()
foreach ($ins in $new) { $gaol.Instructions.Add($ins) }
foreach ($p in $gaol.Parameters) { $p.InstructionIndex = $map[[int]$p.InstructionIndex] }
Invoke-InDeps { $commonFunc.Write($path) }
"common_func: 紫16 监牢事件 $GaolEvent 插入 4 条（标记 $Marker 时不查、不扣石剑钥匙）"
