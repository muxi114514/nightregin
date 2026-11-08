<#
  从游戏加密归档（Game\data*.bdt / dlc01.bdt）取出原版文件，供独立版构建当底（tools\build_vanilla_pkg.ps1）。
  原理：bhd 是 RSA 公钥逐块加密的 BHD5 文件表（每 256 字节解出 255 字节）；条目按路径哈希（小写、/ 开头，h = h*0x85 + c，64 位）查找；
  bdt 里的文件部分区段是 AES-128-ECB 加密，密钥和区段表在 bhd 条目的 AES 偏移处。公钥取自 Smithbox（Andre.Formats）。
  默认输出到 target\vanilla\<路径>，已存在的跳过（-Force 重取）。
#>
param(
    [string]$Game = "$PSScriptRoot\..\..\Game",
    [string]$OutDir = "$PSScriptRoot\..\target\vanilla",
    [string[]]$Paths = @(
        "/chr/c0000.anibnd.dcx",
        "/action/script/c0000.hks",
        "/msg/zhocn/item_dlc01.msgbnd.dcx", "/msg/zhocn/menu_dlc01.msgbnd.dcx",
        "/msg/engus/item_dlc01.msgbnd.dcx", "/msg/engus/menu_dlc01.msgbnd.dcx",
        "/sfx/sfxbnd_commoneffects.ffxbnd.dcx", "/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx",
        "/script/talk/m60_00_00_00.talkesdbnd.dcx", "/script/talk/m12_00_00_00.talkesdbnd.dcx", "/script/talk/m32_00_00_00.talkesdbnd.dcx"
    ),
    [switch]$Force
)
$ErrorActionPreference = "Stop"

# 各归档的 RSA 公钥（PKCS#1 DER 的 base64）
$Keys = @{
    data0 = "MIIBDAKCAQEAz8F9U1V9hgKs40gdzl1ZOf3IBirf6xUEzXtDd6oSEBE6XiYocvABykiK+WMdAaJL7HJ58Gt2xSRxA3t9toCGKMI/3gNAfcR0BV83gsQo0O0dVP0fqyxXlA2pGN5B4IE8aLWPX2cNNFSFKAdjYnzsYSevzef/pgnpV1ZgPf2j2SQwNGSufYeN3Owji8l0K2C0fKIx6gSO0cK9kvTIm8AdpvzZbBkTylT1jF3m8DsSA1OFzFJTdFyZbTRi85M6bmv6rHtvZc5OW21dye7Q6fmLlxOyMetLTu4dpOXjHAAf/LFTbfQpXFr9aXO4O6I7nWDJn7FRzNlLkb8RwSyZ1/KWyQIFALEDsAc="
    data1 = "MIIBDAKCAQEA0E6dtnDmT6d2+VaNkPzomUNv+T6896H//RAaTR2guPACMDNZpAsFvV3MfNcR2BS6Cbxl55MmMWsmsZs1s293MuOdS+c99vmZbNYcXWjx0uJGO+VrRXe43TRzmQFh1uD+Xcq6+wYfTrGyLOdAtmwdDXNvW8jYoFDM7nsuoPKOXKtKd0uz7/MKZYLk1J7pAoBQqw9VD5qi2Ih86zn0VWm5lLMTI0qnutOzpZVDvZWBg/jr4Nbnr/OxPLeJO1tFuRuHUPuBAWtYM/J23MPqqKkQrG5z2r7PexUI744UPdmo3Sn+MqynuxxvV9SEhska6pStzn8R9i94wOKPTQ32HEFuUQIFAP////8="
    data2 = "MIIBDAKCAQEAqpkf9yHnx8k84+WXITLFUW/STypXjZMPuw842pzNHa5L7v9gU4M5hBHwTQs0YIcfnf+mbjqoJYnmYPBblxLjFXgwT4ICJdpnPMY75BwD0Nv28/CvvIsA0QQWOhUeOXnm5BT26dGYi3CHHPvD14F76tJt3TO/CC3fyhdxne9Cra5G87aGTJGv0ImsU0KPCizYX/RHQ2jdJdlB5BHzkMgLhIaEdhC3nhIqMJDNQNGKMo7rRV1tAEGf0zIZ23PGEsPsbVg31nnnRoq338WfD9ArZZG6bM11vlfVcYmrJs7v4vBjKXnYVwVX0rQGIfSNDnaZcEj4tsl04AqnupTdvSrHXwIFANOg6RU="
    data3 = "MIIBCwKCAQEAwm2Rcw4eoP8FgWijxw1X8b9rEVFsVqy7rXWcH2yVm61yYBlzPlTqKqnc2VeqZSh/TLXeFY3+Om2X78RQxZNS3L3OokvD7l/0wqPIpXSSumeeL8UAZm5k7nFA2m2HJfc+F07kNwwCEqhmFs5YQIMnWyIrqnEax/qSncFErLjIYMBMArVnVLE8WqgsD7N8lW937dlUcT2TaPh1HfjavKOSUy/OHM9zaneyDL4NRmDdU8GmNXTSm5kPYoSRCDIvFVj0g5iaXr60eRh0d+40TctoBUdtaoJCPOyRlmkE7qU6Q9FyyvMNbhtfD95d+6IJejNd7kvyV/ISlB37kb2Uh9TavwIEOqKLtw=="
    dlc01 = "MIIBDAKCAQEA1q4MOehlD++h5Ietq9Jk97eGOJL2zDpDcu9Wk6RXK1+R3LycMBQlL/hnPg/qqvcoViA7wLX5GOFr5lo6dtKaQqlBkBqgYHGIdBvioBPZ8BuXAjYr3sm8N0SYC2TNHXmfw6yFC+ePsrl+gNldrO//XXY27hsGgcegfWr6JuQaJti/BOKlGb8ARbKwyIqGc5WiWj/v0tGE1cdPi0fLQRbTrLFaQtx1roQVqsQuJ5zRGTpnj/mhaJtqJ7V0s5gLG5CCevx71lN8m7oyWk2JemzSLvllwv4tjtzrw3jNQtiYb8nzy2SpjibsvX1iRCg5btMSiNPcSeIJ5jX+FUW9LSnrkwIFAKhopbM="
}

. (Join-Path $PSScriptRoot "fs_archive.ps1")

$Game = (Resolve-Path $Game).Path
New-Item -ItemType Directory -Force $OutDir | Out-Null
$OutDir = (Resolve-Path $OutDir).Path
$todo = $Paths | Where-Object { $Force -or -not (Test-Path (Join-Path $OutDir $_.TrimStart('/'))) }
if (-not $todo) { "原版文件已在 $OutDir（-Force 重取）"; return }

$index = @{}; $bhds = @{}
foreach ($name in $Keys.Keys) {
    $bhdPath = Join-Path $Game "$name.bhd"
    if (-not (Test-Path $bhdPath)) { continue }
    $bhds[$name] = [NrArchive]::DecryptBhd($bhdPath, $Keys[$name])
    foreach ($e in [NrArchive]::Parse($bhds[$name], $name)) { $index[$e.Hash] = $e }
}
foreach ($p in $todo) {
    $e = $index[[NrArchive]::Hash($p)]
    if (-not $e) { throw "not found in archives: $p" }
    $bytes = [NrArchive]::Extract($e, $bhds[$e.Archive], (Join-Path $Game "$($e.Archive).bdt"))
    $dst = Join-Path $OutDir $p.TrimStart('/')
    New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
    [IO.File]::WriteAllBytes($dst, $bytes)
    "{0,-46} {1,-6} {2,10}" -f $p, $e.Archive, $bytes.Length
}
