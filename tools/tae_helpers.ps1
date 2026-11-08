<#
  relic_assets.ps1 与 tiered_tae.ps1 共用：加载 ERBM 自带的 Andre.SoulsFormats，读写 BND / DCX，以及 TAE 子弹事件的几个小工具。
  点引用前先设好 $app（ERBM 的 app 目录）与 $deps（Dependencies 目录，KRAK 压缩的 oo2core 在这里）。
#>

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
# 读特效包：有的包是 DFLT 压缩但参数特殊，SoulsFormats 不认，要自己解压（头 0x4C 字节，+0x1C 大端为解压后大小，之后 2 字节 zlib 头）
function Read-Bank([string]$Path) {
    $fs = [IO.File]::OpenRead($Path)
    try {
        $head = New-Object byte[] 0x4E
        [void]$fs.Read($head, 0, $head.Length)
        if ([Text.Encoding]::ASCII.GetString($head, 0x28, 4) -ne "DFLT") { return Read-Bnd $Path }
        $size = ([int]$head[0x1C] -shl 24) -bor ([int]$head[0x1D] -shl 16) -bor ([int]$head[0x1E] -shl 8) -bor [int]$head[0x1F]
        $ms = New-Object IO.MemoryStream $size
        (New-Object IO.Compression.DeflateStream($fs, [IO.Compression.CompressionMode]::Decompress)).CopyTo($ms)
        if ($ms.Length -ne $size) { throw "$Path decompressed to $($ms.Length) bytes, expected $size" }
        [SoulsFormats.BND4]::Read($ms.ToArray())
    } finally { $fs.Dispose() }
}
function Find-File($Bnd, [string]$Suffix) {
    $f = $Bnd.Files | Where-Object { $_.Name.ToLower().EndsWith($Suffix) } | Select-Object -First 1
    if (-not $f) { throw "$Suffix not found" }
    $f
}

# ---- TAE：子弹事件参数 +0 挂点、+8 判定号、+14 状态条件（s16，0 为无条件）----
function Get-Params($Ev) { [byte[]]$Ev.GetParameterBytes($false) }
function Get-Judge($Ev) { [BitConverter]::ToInt32((Get-Params $Ev), 8) }
function Get-State($Ev) { [BitConverter]::ToInt16((Get-Params $Ev), 14) }
function New-BulletEvent($Template, [int]$Judge, [int]$State, [float]$Start, $Group) {
    $ev = $Template.GetClone($false)
    $p = Get-Params $Template
    [BitConverter]::GetBytes([int]$Judge).CopyTo($p, 8)
    [BitConverter]::GetBytes([int16]$State).CopyTo($p, 14)
    $ev.SetParameterBytes($false, $p, $false)
    $ev.StartTime = $Start
    $ev.EndTime = $Start + ($Template.EndTime - $Template.StartTime)
    $ev.Group = $Group
    $ev
}
function Set-State($Ev, [int]$State) {
    $p = Get-Params $Ev
    [BitConverter]::GetBytes([int16]$State).CopyTo($p, 14)
    $Ev.SetParameterBytes($false, $p, $false)
}
# 攻击事件参数 +4 是攻击编号：同一动画里编号相同、时间重叠的攻击共用命中名单，已被打中的目标不会再被打中
# （原版铁之眼遗物的追加攻击用 1，与技艺本体的 0 错开）
function Get-AttackIndex($Ev) { [BitConverter]::ToInt32((Get-Params $Ev), 4) }
function Set-AttackIndex($Ev, [int]$Index) {
    $p = Get-Params $Ev
    [BitConverter]::GetBytes([int]$Index).CopyTo($p, 4)
    $Ev.SetParameterBytes($false, $p, $false)
}
# 跳转表事件（类型 0）参数 +0 是跳转表编号，状态条件同样在 +14
function Get-JumpId($Ev) { [BitConverter]::ToInt32((Get-Params $Ev), 0) }
function Set-JumpId($Ev, [int]$Id) {
    $p = Get-Params $Ev
    [BitConverter]::GetBytes([int]$Id).CopyTo($p, 0)
    $Ev.SetParameterBytes($false, $p, $false)
}
function Remove-Event($Anim, $Ev) {
    $Anim.Events.Remove($Ev) | Out-Null
    if (-not ($Anim.Events | Where-Object { $_.Group -eq $Ev.Group })) { $Anim.EventGroups.Remove($Ev.Group) | Out-Null }
}
function Get-Anim($Tae, [long]$Id) {
    $a = $Tae.Animations | Where-Object { $_.ID -eq $Id } | Select-Object -First 1
    if (-not $a) { throw "animation $Id not found" }
    $a
}
# 按模板事件的组类型新建一个事件组（原版同时触发的不同判定各占一组）
function New-Group($Anim, $Template) {
    $g = $Template.Group.GetClone()
    $Anim.EventGroups.Add($g)
    $g
}
