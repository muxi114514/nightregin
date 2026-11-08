<#
  词条继承的锻造台入口（MMV 整合包与原版独立版共用），生成到 pkg：
    script\talk\<地图>.talkesdbnd.dcx  有锻造台的地图（$AnvilTalks）里"打开强化界面"的子状态机改为先弹列表：强化武器 / 继承词条 / 离开。
                                        m60 大地图 t602006000~t602046000、特殊锻造台 t701006000；m12 / m32 只有锻造台（t602001200 /
                                        t602003200）和商人（t800）两段对话，是第三天首领前的集结点。选继承词条时 SetEventFlag(1996800001, 1)，
                                        由 DLL（src\passive_inherit\entry.rs）截下并打开继承面板，不写入真实旗标，也不会同步给队友
    msg\<语言>\menu_dlc01.msgbnd.dcx    EventTextForTalk 新增两条列表文本；OutDir 里已有（relic_assets.ps1 生成过）就在它上面改
  来源：-TalkDirs 按顺序找，第一个有该文件的为准（MMV 整合包自带 m60，m12 / m32 只有原版，由 tools\extract_vanilla.ps1 导出）。
  只认原版强化子状态机的形状（c1_35 → c1_24(0) → 返回），已改过或形状不同的跳过；每个地图至少要改到一处，否则报错；
  不碰 regulation，联机哈希不受影响。
#>
param(
    [Parameter(Mandatory)] [string[]]$TalkDirs,
    # 其下为 <语言>\menu_dlc01.msgbnd.dcx
    [Parameter(Mandatory)] [string]$MsgSrcDir,
    [Parameter(Mandatory)] [string]$OutDir,
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
# 读写 DCX 时会临时切换当前目录，路径必须先转成绝对路径
$OutDir = (New-Item -ItemType Directory -Force $OutDir).FullName
$MsgSrcDir = (Resolve-Path $MsgSrcDir).Path
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path

# 有锻造台的地图对话脚本（扫过原版全部 21 个对话脚本，只有这三个有打开强化界面的 c1_24）
$AnvilTalks = @("m60_00_00_00", "m12_00_00_00", "m32_00_00_00")
# 与 DLL 的 REQUEST_FLAG 一致
$RequestFlag = 1996800001
# 列表项序号与文本（menu_dlc01 的 EventTextForTalk）；离开沿用原版文本 20000009
$TalkFmg = 33
$Reinforce = @{ Index = 1; Text = 19968010 }
$Inherit = @{ Index = 2; Text = 19968011 }
$Leave = @{ Index = 99; Text = 20000009 }
$Texts = @(
    @{ Id = $Reinforce.Text; zhocn = "强化武器"; engus = "Reinforce weapon" },
    @{ Id = $Inherit.Text; zhocn = "继承词条"; engus = "Inherit passives" }
)

# ---- SoulsFormats ----
[AppDomain]::CurrentDomain.add_AssemblyResolve({ param($s, $e) $p = [IO.Path]::Combine($app, $e.Name.Split(',')[0] + '.dll'); if ([IO.File]::Exists($p)) { [Reflection.Assembly]::LoadFrom($p) } else { $null } }) | Out-Null
[Reflection.Assembly]::LoadFrom("$app\Andre.SoulsFormats.dll") | Out-Null

# DCX 的 KRAK 压缩要从当前目录加载 oo2core
function Invoke-InDeps([scriptblock]$Block) {
    $old = [IO.Directory]::GetCurrentDirectory()
    [IO.Directory]::SetCurrentDirectory($deps)
    try { & $Block } finally { [IO.Directory]::SetCurrentDirectory($old) }
}
function Read-Bnd([string]$Path) { Invoke-InDeps { [SoulsFormats.BND4]::Read($Path) } }
function Write-Bnd($Bnd, [string]$Path) {
    New-Item -ItemType Directory -Force (Split-Path $Path) | Out-Null
    Invoke-InDeps { $Bnd.Write($Path) }
}

# ---- ESD 表达式字节码：0x00~0x7F 为小整数（减 0x40），0x82 + int32，0x84+n 调用 n 个参数的函数，0x95 ==，0xA1 结束 ----
function Push([int]$v) { if ($v -ge -64 -and $v -le 63) { , [byte]($v + 0x40) } else { [byte[]](@(0x82) + [BitConverter]::GetBytes($v)) } }
function Arg([int]$v) { [byte[]](@(Push $v) + 0xA1) }
$Always = Arg 1
# 列表菜单（f59 菜单类型 1）已关闭：not (f59(1, 0) == 1 and f58(0) == 0)，照抄原版对话脚本
$ListClosed = [byte[]](0x7B, 0x41, 0x40, 0x86, 0xA6, 0x41, 0x95, 0xA6, 0x7A, 0x40, 0x85, 0xA6, 0x40, 0x95, 0x98, 0x40, 0x95, 0xA1)
# f23()（选中的列表项序号）== n
function Picked([int]$n) { [byte[]](@(0x57, 0x84, 0xA6) + @(Push $n) + @(0x95, 0xA1)) }

function New-Cmd([int]$Bank, [int]$Id, [int[]]$Values) {
    [byte[][]]$argBytes = @(foreach ($v in $Values) { , (Arg $v) })
    [SoulsFormats.ESD+CommandCall]::new($Bank, $Id, $argBytes)
}
function New-State([object[]]$Entry, [object[]]$Conds) {
    $st = [SoulsFormats.ESD+State]::new()
    foreach ($c in $Entry) { $st.EntryCommands.Add($c) }
    foreach ($c in $Conds) { $st.Conditions.Add([SoulsFormats.ESD+Condition]::new([long]$c[0], [byte[]]$c[1])) }
    $st
}
function Has-Cmd($State, [int]$Bank, [int]$Id) { [bool]($State.EntryCommands | Where-Object { $_.CommandBank -eq $Bank -and $_.CommandID -eq $Id }) }
function Find-State($States, [scriptblock]$Pred) {
    foreach ($k in $States.Keys) { if (& $Pred $States[$k]) { return $k } }
    $null
}

# 原版强化子状态机：0 → c1_35()（准备）→ c1_24(0) 打开强化界面、等关闭 → … → 返回。入口改指向新增的列表状态
function Edit-ReinforceGroup($States) {
    $prep = Find-State $States { param($s) Has-Cmd $s 1 35 }
    $open = Find-State $States { param($s) Has-Cmd $s 1 24 }
    $done = Find-State $States { param($s) [bool]($s.Conditions | Where-Object { $_.PassCommands | Where-Object { $_.CommandBank -eq 7 } }) }
    $entry = $States[[long]0]
    if ($null -eq $prep -or $null -eq $open -or $null -eq $done -or -not $entry -or $entry.Conditions.Count -ne 1 -or $entry.Conditions[0].TargetState -ne $prep) { return $false }
    if ($null -ne (Find-State $States { param($s) Has-Cmd $s 1 10 })) { return $false }
    $base = ($States.Keys | Measure-Object -Maximum).Maximum + 1
    $clear, $add, $show, $branch, $request = $base, ($base + 1), ($base + 2), ($base + 3), ($base + 4)
    # c1_20 清空列表、c1_110（原版列表前都调用）、c1_19 添加项、c1_10(1) 显示列表，与原版局内对话 t703006000 的写法一致
    $States.Add([long]$clear, (New-State @((New-Cmd 1 20 @()), (New-Cmd 1 110 @())) @(, @($add, $Always))))
    $States.Add([long]$add, (New-State @((New-Cmd 1 19 @($Reinforce.Index, $Reinforce.Text, -1)), (New-Cmd 1 19 @($Inherit.Index, $Inherit.Text, -1)),
        (New-Cmd 1 19 @($Leave.Index, $Leave.Text, -1))) @(, @($show, $Always))))
    $States.Add([long]$show, (New-State @(, (New-Cmd 1 10 @(1))) @(, @($branch, $ListClosed))))
    $States.Add([long]$branch, (New-State @() @(@($prep, (Picked $Reinforce.Index)), @($request, (Picked $Inherit.Index)), @($done, $Always))))
    # c1_11 = SetEventFlag(旗标, 值)
    $States.Add([long]$request, (New-State @(, (New-Cmd 1 11 @($RequestFlag, 1))) @(, @($done, $Always))))
    $entry.Conditions[0].TargetState = $clear
    $true
}

# ---- 1. 对话脚本 ----
foreach ($map in $AnvilTalks) {
    $name = "$map.talkesdbnd.dcx"
    $src = $TalkDirs | ForEach-Object { Join-Path $_ $name } | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $src) { throw "$name not found in $($TalkDirs -join ' / ')" }
    $talk = Read-Bnd (Resolve-Path $src).Path
    $patched = @()
    foreach ($file in $talk.Files) {
        $esd = [SoulsFormats.ESD]::Read($file.Bytes.ToArray())
        $changed = $false
        foreach ($g in @($esd.StateGroups.Keys)) {
            if (Edit-ReinforceGroup $esd.StateGroups[$g]) {
                $patched += "$([IO.Path]::GetFileNameWithoutExtension($file.Name))/$g"
                $changed = $true
            }
        }
        if ($changed) { $file.Bytes = [byte[]]$esd.Write() }
    }
    if (-not $patched) { throw "no anvil reinforce state machine found in $src" }
    Write-Bnd $talk (Join-Path $OutDir "script\talk\$name")
    "script\talk\${map}: 改了 $($patched.Count) 个强化子状态机（$($patched -join '、')）"
}

# ---- 2. 列表文本 ----
foreach ($lang in "zhocn", "engus") {
    $out = Join-Path $OutDir "msg\$lang\menu_dlc01.msgbnd.dcx"
    $src = if (Test-Path $out) { $out } else { Join-Path $MsgSrcDir "$lang\menu_dlc01.msgbnd.dcx" }
    $msg = Read-Bnd (Resolve-Path $src).Path
    $file = $msg.Files | Where-Object { $_.ID -eq $TalkFmg } | Select-Object -First 1
    if (-not $file) { throw "$lang\menu_dlc01 FMG $TalkFmg not found" }
    $fmg = [SoulsFormats.FMG]::Read($file.Bytes.ToArray())
    $added = 0
    foreach ($t in $Texts) {
        $exist = $fmg.Entries | Where-Object { $_.ID -eq $t.Id } | Select-Object -First 1
        if ($exist -and $exist.Text -ne $t[$lang]) { throw "$lang\menu_dlc01 FMG $TalkFmg entry $($t.Id) already used: $($exist.Text)" }
        if (-not $exist) { $fmg.Entries.Add((New-Object SoulsFormats.FMG+Entry($fmg, $t.Id, $t[$lang]))); $added++ }
    }
    $sorted = @($fmg.Entries | Sort-Object ID)
    $fmg.Entries.Clear()
    foreach ($e in $sorted) { $fmg.Entries.Add($e) }
    $file.Bytes = [byte[]]$fmg.Write()
    Write-Bnd $msg $out
    "$lang\menu_dlc01.msgbnd.dcx: 新增列表文本 $added 条"
}
