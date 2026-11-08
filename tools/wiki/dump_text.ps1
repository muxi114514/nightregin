<#
  图鉴导出第一步：把整合包的文本包（item / menu）按「原版 → MMV 底包 → pkg_mmv」顺序合并，导出成 JSON（只读）。
  产物 <Out>\text.json：{ zhocn: { "item/WeaponName": { id: 文本 } }, engus: {...}, src: { "item/WeaponName": { id: 来源序号 } }, fmgIds: { "item/WeaponName": 11 } }
  src 只记中文、且不是来自原版（序号 0）的条目，用来判断哪些文本是 MMV / 整合包加的。
#>
param(
    [string[]]$MsgDirs = @(
        "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\NRStatAlloc\target\vanilla\msg",
        "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\MMV\me3\mod\msg",
        "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\MMV\me3\StatAlloc\pkg_mmv\msg"
    ),
    [string]$Erbm = "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\me3\NightreignPLUS\ERBMv1.1\Files",
    [Parameter(Mandatory)] [string]$Out
)
$ErrorActionPreference = "Stop"
$app = (Resolve-Path "$Erbm\app").Path
$deps = (Resolve-Path "$Erbm\Dependencies").Path
$env:PATH = "$Erbm\x64;$deps;" + $env:PATH
. "D:\steam\steamapps\common\ELDEN RING NIGHTREIGN\NRStatAlloc\tools\tae_helpers.ps1"
Add-Type -AssemblyName System.Web.Extensions

# 直接 ::new()，函数输出会被包成 PSObject，序列化时报循环引用
$D = [System.Collections.Generic.Dictionary[string,object]]
$root = $D::new()
foreach ($k in "zhocn", "engus", "src", "fmgIds") { $root[$k] = $D::new() }

for ($i = 0; $i -lt $MsgDirs.Count; $i++) {
    foreach ($lang in "zhocn", "engus") {
        foreach ($kind in "item", "menu") {
            $path = Join-Path $MsgDirs[$i] "$lang\$($kind)_dlc01.msgbnd.dcx"
            if (-not (Test-Path $path)) {
                # 最后一层是整合包自己的文本，缺了导出来的就是错的
                if ($i -eq $MsgDirs.Count - 1) { throw "找不到文本包：$path" }
                Write-Warning "skip $path"
                continue
            }
            $bnd = Read-Bnd $path
            foreach ($f in $bnd.Files) {
                $key = "$kind/" + [IO.Path]::GetFileNameWithoutExtension($f.Name)
                $root.fmgIds[$key] = [int]$f.ID
                if (-not $root[$lang].ContainsKey($key)) { $root[$lang][$key] = $D::new() }
                if ($lang -eq "zhocn" -and -not $root.src.ContainsKey($key)) { $root.src[$key] = $D::new() }
                $fmg = [SoulsFormats.FMG]::Read($f.Bytes)
                foreach ($e in $fmg.Entries) {
                    if (-not $e.Text) { continue }
                    $root[$lang][$key]["$($e.ID)"] = $e.Text
                    if ($lang -eq "zhocn") {
                        if ($i -gt 0) { $root.src[$key]["$($e.ID)"] = $i } else { [void]$root.src[$key].Remove("$($e.ID)") }
                    }
                }
            }
            "{0} {1} {2}: {3} fmg" -f $i, $lang, $kind, $bnd.Files.Count
        }
    }
}
$ser = New-Object System.Web.Script.Serialization.JavaScriptSerializer
$ser.MaxJsonLength = [int]::MaxValue
New-Item -ItemType Directory -Force $Out | Out-Null
[IO.File]::WriteAllText((Join-Path $Out "text.json"), $ser.Serialize($root), (New-Object System.Text.UTF8Encoding $false))
"written $Out\text.json"
