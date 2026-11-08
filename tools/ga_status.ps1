<#
  状态显示：效果行配图标 + 自己的文本（spEffectTextId_1 → menu_dlc01 的效果名 214 / 来源 215），就能进「特殊效果」详情页。
  左上角状态栏还要有限时长：不限时的只进详情页（实测 4 个不限时的探针都不上状态栏，30 秒的冷却能上）。
  ga_port.ps1 调 Add-ShownEffects 改已带过来的效果行，最后调 Write-StatusText 写文本；层数效果由 ga_versatility.ps1 调 Set-StatusShown。
#>
$FmgEffectName = 214
$FmgEffectInfo = 215
# 不明确显示的冷却统一用这个图标（绝招后补满的冷却 9971213 同款）
$CooldownIcon = 20497
$ShownEffects = @(
    # 次元斩绝的冷却：死亡不清，否则死一次就能重置
    @{ Id = 10020083; Icon = $CooldownIcon; KeepOnDeath = $true
        Name = @{ zhocn = "次元斩绝（冷却中）"; engus = "Judgement Cut End (Cooldown)" }
        Info = @{ zhocn = "阎魔刀带来的效果"; engus = "Effect of Yamato" } }
)
$statusText = @()

function Set-StatusShown($Row, [int]$Icon, [hashtable]$Name, [hashtable]$Info) {
    Set-Cell $Row "iconId" $Icon
    Set-Cell $Row "spEffectTextId_1" $Row.ID
    $script:statusText += @{ Id = [int]$Row.ID; Name = $Name; Info = $Info }
}
function Add-ShownEffects {
    foreach ($e in $ShownEffects) {
        $row = Get-Row $nr.SpEffectParam $e.Id
        if (-not $row) { throw "effect $($e.Id) not found" }
        Set-StatusShown $row $e.Icon $e.Name $e.Info
        if ($e.KeepOnDeath) { Set-Cell $row "dontDeleteOnDead" 1 }
    }
}
function Write-StatusText([string]$PkgPath, $Languages) {
    foreach ($lang in $Languages) {
        $path = "$PkgPath\msg\$lang\menu_dlc01.msgbnd.dcx"
        $msg = Read-Bnd $path
        foreach ($target in @(@{ Fmg = $FmgEffectName; Key = "Name" }, @{ Fmg = $FmgEffectInfo; Key = "Info" })) {
            $file = $msg.Files | Where-Object { $_.ID -eq $target.Fmg } | Select-Object -First 1
            if (-not $file) { throw "$lang menu_dlc01 FMG $($target.Fmg) not found" }
            $fmg = [SoulsFormats.FMG]::Read($file.Bytes)
            foreach ($t in $statusText) {
                if ($fmg.Entries | Where-Object { $_.ID -eq $t.Id }) { throw "$lang FMG $($target.Fmg) entry $($t.Id) already used" }
                $fmg.Entries.Add((New-Object SoulsFormats.FMG+Entry($fmg, $t.Id, $t[$target.Key][$lang])))
            }
            $sorted = @($fmg.Entries | Sort-Object ID)
            $fmg.Entries.Clear()
            foreach ($e in $sorted) { $fmg.Entries.Add($e) }
            $file.Bytes = [byte[]]$fmg.Write()
        }
        Write-Bnd $msg $path
        "$lang\menu_dlc01：状态文本 $($statusText.Count) 条"
    }
}
