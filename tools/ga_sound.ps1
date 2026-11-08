<#
  把 GoldenAge 武器的专属音频包放到 DLL 旁的 sound_banks（DLL 启动时全部加载，见 src\sound_banks.rs）。
  不能放进 pkg 的 sd\：cs_c7800.bnk 同名会顶掉黑夜君临 c7800 自己的包。两边 Wwise 包版本都是 135，文件原样拷贝。
    阎魔刀：cs_c7800，招式表的 c780006xxx / c780008002 都在这个包里，包头编号是 mod 自定义的，不和原版撞。
#>
param(
    [Parameter(Mandatory)] [string]$StatAllocDir,
    [string]$GaRoot = "D:\steam\steamapps\common\ELDEN RING\Game\GA",
    [string[]]$Banks = @("cs_c7800")
)
$ErrorActionPreference = "Stop"
$folder = Join-Path (Resolve-Path $StatAllocDir).Path "sound_banks"
New-Item -ItemType Directory -Force $folder | Out-Null
foreach ($name in $Banks) {
    Copy-Item "$GaRoot\sd\$name.bnk" "$folder\$name.bnk" -Force
    "音频包 $name：$((Get-Item "$folder\$name.bnk").Length) 字节"
}
