<#
  把 GoldenAge 武器用到、黑夜君临没有的特效加进 pkg 的 commoneffects_dlc01（pkg 里要已有这个包，relic_sfx.ps1 之后运行）。
  FXR 转成黑夜君临格式（tools\fxr_nr.ps1），资源清单原样拷贝。
  带哪些资源（贴图 tex\*.tpf、模型 model\*.flver、动画 hkx\*.anibnd），按编号判断，不能只看资源清单：
    清单写 s01000_a.tif，包里文件可能叫 s01000.tpf；清单还列着特效没用的贴图，特效用到的模型和贴图清单里又可能没有。
    所以按 FXR 动作里的编号位置取（Get-FxrResourceNumbers；主贴图和模型是属性里的浮点常量，GA 自制特效的清单还常常是空的），带上该编号的全部文件，
    再加 4 字节对齐的大整数（不小于 $MinResourceId；偶尔把别的数当编号只是多带一点文件，不影响特效），
    和清单里 tif / tpf 的编号（只带贴图）。清单里的 sib 模型 FXR 没写就是没用，不带。模型还要带材质引用的贴图。
  来源优先级：GA 特效包、MMV 其他特效包（$MmvExtra）、$ErFxDir（tools\ga_er_fx.ps1 从法环归档取出）。黑夜君临常驻包已有同名文件的不带。
#>
param(
    [Parameter(Mandatory)] [string]$Pkg,
    [string]$GaRoot = "D:\steam\steamapps\common\ELDEN RING\Game\GA",
    [string]$ErFxDir = "$PSScriptRoot\..\target\ga_er_fx",
    [string]$VanillaCommon = "$PSScriptRoot\..\target\vanilla\sfx\sfxbnd_commoneffects.ffxbnd.dcx",
    [string]$MmvSfx = "$PSScriptRoot\..\..\me3\MMV\me3\mod\sfx",
    [string[]]$MmvExtra = @("sfxbnd_c1260", "sfxbnd_c8500", "sfxbnd_c9013"),
    [string]$Erbm = "$PSScriptRoot\..\..\me3\NightreignPLUS\ERBMv1.1\Files"
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. (Join-Path $PSScriptRoot "tae_helpers.ps1")
. (Join-Path $PSScriptRoot "fxr_nr.ps1")

# 阎魔刀的招式表、子弹、外观行用到而黑夜君临没有的特效
$EffectIds = @(647335, 900613402, 900647280, 900650686, 900654115, 900654152, 900671133, 900980003,
    901441200, 901441201, 901441204, 901441206, 901441210, 901441211, 901441212, 901441213, 901441216, 901441218, 901441219,
    901441220, 901441221, 901441223, 901441224, 901441225, 901441226, 901441227, 901441231, 901441232, 901441235, 901441236, 901441238,
    901441240, 901441241, 901441242, 901441243, 901441244, 901441245, 901441246, 901441247, 901441255, 901441260, 901441261, 901441262,
    990523092, 990523172, 990523173, 999450490, 999450491, 999523042, 999523043, 999523044, 999523045, 999523046, 999600000, 999600115, 999646811)
# 特效包里的编号段：FXR 从 0、贴图 100000、模型 200000、动画 300000、资源清单 400000
$ListIdBase = 400000
$FolderByExt = @{ ".tpf" = "tex"; ".flver" = "model"; ".anibnd" = "hkx" }
$ResourcePattern = '^s(\d+)(_[a-z0-9]+)?\.(tpf|flver|anibnd)$'
$ListedPattern = '(?i)[as](\d+)(_[a-z0-9]+)?\.(tif|tpf)'
$TexturePathPattern = '(?i)s(\d+)(_[a-z0-9]+)?\.(tif|dds|tga)'
# 小于它的整数不当资源编号（FXR 里到处是小整数）；小编号贴图靠资源清单带
$MinResourceId = 1000
# FXR 头：引用的其他 FXR 列表的偏移和数量（第 28、29 个 int）
$RefOffsetAt = 112
$RefCountAt = 116

function Get-FxId([string]$Name) { if ($Name -match "f0*(\d+)\.(fxr|ffxreslist)$") { [int64]$Matches[1] } else { -1 } }
function Get-Leaf([string]$Name) { [IO.Path]::GetFileName($Name.ToLower()) }
function Get-FxRefs([byte[]]$Bytes) {
    $at = [BitConverter]::ToInt32($Bytes, $RefOffsetAt); $n = [BitConverter]::ToInt32($Bytes, $RefCountAt)
    for ($k = 0; $k -lt $n; $k++) { [int64][BitConverter]::ToInt32($Bytes, $at + 4 * $k) }
}

$pkgRoot = (Resolve-Path $Pkg).Path
$bankPath = "$pkgRoot\sfx\sfxbnd_commoneffects_dlc01.ffxbnd.dcx"
$bank = Read-Bank $bankPath
$wanted = @{}; foreach ($id in $EffectIds) { $wanted[[int64]$id] = 1 }

# 黑夜君临已有的特效和资源（这几个包都是常驻加载的）
$haveFx = @{}; $haveRes = @{}; $haveNumber = @{}
function Add-Known($Bnd) {
    foreach ($f in $Bnd.Files) {
        $leaf = Get-Leaf $f.Name
        if ($leaf.EndsWith(".fxr")) { $haveFx[(Get-FxId $leaf)] = 1 }
        elseif ($leaf -match $ResourcePattern) { $haveRes[$leaf] = 1; $haveNumber[[int]$Matches[1]] = 1 }
    }
}
Add-Known $bank
foreach ($path in @($VanillaCommon, "$MmvSfx\sfxbnd_c0000.ffxbnd.dcx")) { $other = Read-Bank (Resolve-Path $path).Path; Add-Known $other; $other = $null; [GC]::Collect() }
foreach ($id in $EffectIds) { if ($haveFx.ContainsKey([int64]$id)) { throw "f$id already exists (repeated run?)" } }

# 资源来源：编号 -> 文件名 -> 包里的文件或磁盘路径（先来先得，决定优先级）
$pool = @{}
function Add-Pool([string]$Leaf, $Source) {
    if ($Leaf -match $ResourcePattern) {
        $number = [int]$Matches[1]
        if (-not $pool.ContainsKey($number)) { $pool[$number] = @{} }
        if (-not $pool[$number].ContainsKey($Leaf)) { $pool[$number][$Leaf] = $Source }
    }
}
function Get-Payload($Source) { if ($Source -is [string]) { [IO.File]::ReadAllBytes($Source) } else { $Source.Bytes.ToArray() } }

$fxFiles = @{}; $listFiles = @{}
foreach ($name in "sfxbnd_commoneffects", "sfxbnd_commoneffects_dlc01", "sfxbnd_c0000") {
    $src = Read-Bank "$GaRoot\sfx\$name.ffxbnd.dcx"
    foreach ($f in $src.Files) {
        $leaf = Get-Leaf $f.Name
        if ($leaf.EndsWith(".fxr") -or $leaf.EndsWith(".ffxreslist")) {
            $id = Get-FxId $leaf
            if ($wanted.ContainsKey($id)) {
                if ($leaf.EndsWith(".fxr")) { if (-not $fxFiles.ContainsKey($id)) { $fxFiles[$id] = [byte[]]$f.Bytes.ToArray() } }
                elseif (-not $listFiles.ContainsKey($id)) { $listFiles[$id] = [byte[]]$f.Bytes.ToArray() }
            }
        } else { Add-Pool $leaf $f }
    }
}
foreach ($id in $EffectIds) { if (-not $fxFiles.ContainsKey([int64]$id) -or -not $listFiles.ContainsKey([int64]$id)) { throw "f$id not found in GoldenAge" } }
foreach ($name in $MmvExtra) { $src = Read-Bank (Resolve-Path "$MmvSfx\$name.ffxbnd.dcx").Path; foreach ($f in $src.Files) { Add-Pool (Get-Leaf $f.Name) $f } }
if (Test-Path $ErFxDir) { foreach ($file in Get-ChildItem $ErFxDir -File) { Add-Pool $file.Name.ToLower() $file.FullName } }

# ---- 写进 dlc01 ----
$sample = $bank.Files | Where-Object { $_.Name -like "*.fxr" } | Select-Object -First 1
$flags = $sample.Flags
$prefix = $sample.Name.Substring(0, $sample.Name.ToLower().LastIndexOf("\effect\") + 1)
function Add-BankFile([string]$Ext, [string]$RelName, [int]$Id, [byte[]]$Bytes) {
    $file = New-Object SoulsFormats.BinderFile($flags, "$prefix$RelName", $Bytes)
    $file.ID = $Id
    $last = -1
    for ($k = $bank.Files.Count - 1; $k -ge 0; $k--) { if ($bank.Files[$k].Name.ToLower().EndsWith($Ext)) { $last = $k; break } }
    if ($last -lt 0) { throw "no $Ext in the bank" }
    $bank.Files.Insert($last + 1, $file)
}
function Get-NextId([string]$Ext) { ($bank.Files | Where-Object { $_.Name.ToLower().EndsWith($Ext) } | Measure-Object ID -Maximum).Maximum + 1 }

$added = 0; $bytesAdded = 0; $unknown = @{}; $badRefs = @{}
$bySource = @{ GA = 0; Other = 0 }
# 把一个编号的文件带进包；$Only 限定扩展名。模型的材质贴图接着带
function Add-Resources([int]$Number, [string]$Only) {
    if (-not $pool.ContainsKey($Number)) { return }
    foreach ($leaf in @($pool[$Number].Keys)) {
        $ext = [IO.Path]::GetExtension($leaf)
        if ($Only -and $ext -ne $Only) { continue }
        if ($haveRes.ContainsKey($leaf)) { continue }
        [byte[]]$payload = Get-Payload $pool[$Number][$leaf]
        Add-BankFile $ext "$($FolderByExt[$ext])\$leaf" (Get-NextId $ext) $payload
        $haveRes[$leaf] = 1; $haveNumber[$Number] = 1
        $script:added++; $script:bytesAdded += $payload.Length
        if ($ext -eq ".flver") {
            try {
                foreach ($mat in [SoulsFormats.FLVER2]::Read($payload).Materials) {
                    foreach ($tex in $mat.Textures) {
                        if ($tex.Path -match $TexturePathPattern) { Add-Resources ([int]$Matches[1]) ".tpf" }
                    }
                }
            } catch { "  模型 $leaf 读材质失败: $($_.Exception.Message)" }
        }
    }
}

$fxIndex = Get-NextId ".fxr"
$curves = 0; $padded = 0
foreach ($id in $EffectIds) {
    [int64]$key = $id
    [byte[]]$fxr = $fxFiles[$key]
    foreach ($r in Get-FxRefs $fxr) { if (-not $haveFx.ContainsKey($r) -and -not $wanted.ContainsKey($r)) { $badRefs["$key->$r"] = 1 } }
    $converted = ConvertTo-NrFxr $fxr ([int]$key)
    $curves += $converted.Curves; $padded += $converted.Padded
    Add-BankFile ".fxr" ("effect\f{0:D9}.fxr" -f $key) $fxIndex $converted.Bytes
    Add-BankFile ".ffxreslist" ("ResourceList\f{0:D9}.ffxreslist" -f $key) ($ListIdBase + $fxIndex) $listFiles[$key]
    $fxIndex++
    foreach ($used in Get-FxrResourceNumbers $converted.Bytes) {
        Add-Resources $used ""
        if (-not $haveNumber.ContainsKey($used)) { $unknown["s$used"] = 1 }
    }
    for ($at = 0; $at + 4 -le $fxr.Length; $at += 4) {
        $value = [BitConverter]::ToInt32($fxr, $at)
        if ($value -ge $MinResourceId -and $pool.ContainsKey($value)) { Add-Resources $value "" }
    }
    foreach ($m in [regex]::Matches([Text.Encoding]::UTF8.GetString($listFiles[$key]), $ListedPattern)) {
        $number = [int]$m.Groups[1].Value
        if ($pool.ContainsKey($number)) { Add-Resources $number ".tpf" }
        if (-not $haveNumber.ContainsKey($number)) { $unknown[$m.Value.ToLower()] = 1 }
    }
}
Write-Bnd $bank $bankPath
"特效 $($EffectIds.Count) 个写进 dlc01（转换曲线 $curves 处、补字段 $padded 个）；新增资源 $added 个（$([Math]::Round($bytesAdded / 1048576, 1)) MB 未压缩）；包大小 $([Math]::Round((Get-Item $bankPath).Length / 1048576, 1)) MB"
if ($badRefs.Count) { "引用的 FXR 不存在: $($badRefs.Keys -join ' ')" }
if ($unknown.Count) { "特效或清单写了、所有来源都没有的资源: $(($unknown.Keys | Sort-Object) -join ' ')" }

# 回读：新特效都在，编号不重复，资源文件都读得出
$check = Read-Bank $bankPath
$ids = @{}; foreach ($f in $check.Files) { if ($ids.ContainsKey($f.ID)) { throw "duplicate file id $($f.ID)" }; $ids[$f.ID] = 1 }
$names = @{}; foreach ($f in $check.Files) { $names[$f.Name.ToLower()] = 1 }
foreach ($id in $EffectIds) { if (-not $names.ContainsKey(("$prefix" + ("effect\f{0:D9}.fxr" -f [int64]$id)).ToLower())) { throw "self-check: f$id missing" } }
foreach ($f in $check.Files) {
    if ($f.Name -match "f0*(\d+)\.fxr$" -and $wanted.ContainsKey([int64]$Matches[1])) {
        [byte[]]$written = $f.Bytes.ToArray()
        if ($written[$FxrFlagOffset] -ne 1) { throw "self-check: $($f.Name) flag" }
        if ([BitConverter]::ToInt32($written, $FxrIdOffset) -ne [int]$Matches[1]) { throw "self-check: $($f.Name) internal id" }
        $left = @{ Curves = 0; Padded = 0 }
        Update-FxrContainer (Read-FxrLoose $written).Fxr.RootContainer $left
        if ($left.Curves -or $left.Padded) { throw "self-check: $($f.Name) still has old-format data" }
    }
}
"回读: 文件 $($check.Files.Count)，编号无重复，新特效都是黑夜君临格式"
