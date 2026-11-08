<#
  启动器构建：前端（字体子集 + vite）→ Rust release → 复制到 launcher\dist → 检查导入表 → 冒烟运行
  用法：powershell -ExecutionPolicy Bypass -File launcher\build.ps1
  CARGO_TARGET_DIR 取环境变量 NRGA_TARGET_DIR，默认 D:\DevTools\nrga-launcher-target
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '..\tools\build_env.ps1')
$env:CARGO_TARGET_DIR = if ($env:NRGA_TARGET_DIR) { $env:NRGA_TARGET_DIR } else { 'D:\DevTools\nrga-launcher-target' }
if ($env:CARGO_TARGET_DIR.Contains(' ')) {
    throw "CARGO_TARGET_DIR 不能含空格（windres 预处理会在空格处切断路径）：$env:CARGO_TARGET_DIR"
}

# 运行原生命令，只按退出码判成败（PS 5.1 可能把 stderr 输出包装成错误记录，不能让它中断脚本）
function Invoke-Native([string]$What, [scriptblock]$Command) {
    $ErrorActionPreference = 'Continue'
    # 先清空：命令没能启动时 $LASTEXITCODE 不会更新，不能沿用上一条命令的退出码
    $global:LASTEXITCODE = $null
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$What 失败（退出码 $LASTEXITCODE）" }
}

$ui = Join-Path $PSScriptRoot 'ui'
$dist = Join-Path $PSScriptRoot 'dist'
$release = Join-Path $env:CARGO_TARGET_DIR 'release'

# 1. 前端（tauri 编译时内嵌 ui\dist）
Push-Location $ui
try {
    if (Test-Path 'package-lock.json') { Invoke-Native 'npm ci' { npm.cmd ci } }
    else { Invoke-Native 'npm install' { npm.cmd install } }
    # 字体子集：index.html、src 下全部 .ts，再加 core / app 的 .rs（Issue.detail、失败消息等来自 Rust 的文字）
    # 拼成临时文本，按其中的字裁出 Noto Serif SC
    $text = New-TemporaryFile
    try {
        $sources = @(Get-Item 'index.html') + @(Get-ChildItem 'src' -Recurse -Filter '*.ts' -File)
        foreach ($crate in 'core', 'app') {
            $sources += @(Get-ChildItem (Join-Path $PSScriptRoot "$crate\src") -Recurse -Filter '*.rs' -File)
        }
        # 图鉴的文字也要进子集；还没导出时只提示，动态文字会回退到系统字体
        $wiki = Join-Path $PSScriptRoot '..\..\me3\MMV\me3\launcher\data\wiki.json'
        if (Test-Path $wiki) { $sources += @(Get-Item $wiki) }
        else { Write-Warning "找不到图鉴数据，字体子集不含图鉴文字：$wiki" }
        $all = ($sources | ForEach-Object { [System.IO.File]::ReadAllText($_.FullName) }) -join "`n"
        [System.IO.File]::WriteAllText($text.FullName, $all, (New-Object System.Text.UTF8Encoding $false))
        $subset = Join-Path $PSScriptRoot '..\tools\launcher\subset_font.py'
        $font = Join-Path $env:SystemRoot 'Fonts\NotoSerifSC-VF.ttf'
        Invoke-Native '字体子集' { python $subset $font $text.FullName 'public\fonts\serif_sub.woff' }
    } finally { Remove-Item $text.FullName -ErrorAction SilentlyContinue }
    Invoke-Native 'npm run build' { npm.cmd run build }
} finally { Pop-Location }

# 2. Rust
Push-Location $PSScriptRoot
try { Invoke-Native 'cargo build' { cargo build --release -p launcher } }
finally { Pop-Location }

# 3. 发布物：exe 与 WebView2Loader.dll 放同一目录
New-Item -ItemType Directory -Force $dist | Out-Null
foreach ($name in 'nrga-launcher.exe', 'WebView2Loader.dll') {
    Copy-Item (Join-Path $release $name) $dist -Force
}
$exe = Join-Path $dist 'nrga-launcher.exe'

# 4. 导入表：除 WebView2Loader.dll 外只允许 System32 里有的 DLL 和 api-ms-win-*（$w64 来自 build_env.ps1）
$objdump = Join-Path $w64 'objdump.exe'
$dump = Invoke-Native 'objdump' { & $objdump -p $exe }
$dlls = $dump | ForEach-Object { if ($_ -match '^\s*DLL Name:\s*(\S+)') { $Matches[1] } } | Sort-Object -Unique
if (-not $dlls) { throw 'objdump 没列出任何导入 DLL，无法检查导入表' }
$system32 = Join-Path $env:SystemRoot 'System32'
$bad = @($dlls | Where-Object {
    $_ -ne 'WebView2Loader.dll' -and $_ -notlike 'api-ms-win-*' -and -not (Test-Path (Join-Path $system32 $_))
})
if ($bad.Count -gt 0) { throw "导入了 System32 里没有的 DLL：$($bad -join ', ')" }
Write-Host "导入表检查通过：$($dlls -join ', ')"

# 5. 冒烟：窗口始终隐藏，前端就绪即以 0 退出（exe 自带 30 秒兜底，这里再留 60 秒上限）
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $exe
$psi.Arguments = '--smoke'
$psi.WorkingDirectory = $dist
$psi.UseShellExecute = $false
$proc = [System.Diagnostics.Process]::Start($psi)
if (-not $proc.WaitForExit(60000)) {
    $proc.Kill()
    throw '冒烟运行 60 秒仍未退出'
}
if ($proc.ExitCode -ne 0) { throw "冒烟运行失败（退出码 $($proc.ExitCode)）" }
Write-Host '冒烟运行通过'

Write-Output 'BUILD OK'
