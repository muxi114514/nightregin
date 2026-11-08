<#
  编译环境（由 deploy.ps1 点引用，也可手动 . .\tools\build_env.ps1 后运行 cargo test）：
    Rust GNU 工具链 + w64devkit（ImGui 的 C++ 编译、windows 0.62 需要的 as / dlltool）
    依赖缓存 CARGO_HOME 放 D 盘；rustup 工具链仍在用户目录
  路径可用同名环境变量覆盖
#>
$w64 = if ($env:NR_W64DEVKIT) { $env:NR_W64DEVKIT } else { "D:\DevTools\w64devkit\bin" }
$cargoHome = if ($env:NR_CARGO_HOME) { $env:NR_CARGO_HOME } else { "D:\DevTools\cargo-home" }
$rustupBin = "$env:USERPROFILE\.cargo\bin"

if (-not (Test-Path "$w64\g++.exe")) { throw "w64devkit not found: $w64" }
New-Item -ItemType Directory -Force $cargoHome | Out-Null
$env:CARGO_HOME = $cargoHome
$env:RUSTUP_HOME = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { "$env:USERPROFILE\.rustup" }
# w64devkit 放在最后，避免它自带的 busybox find/sort 等顶替系统命令
$env:PATH = "$rustupBin;" + $env:PATH + ";$w64"

# w64devkit 的 GCC 只有静态库，异常展开已并入 libgcc.a，没有单独的 libgcc_eh.a；
# Rust 标准库链接时固定要 -lgcc_eh，提供一个空的同名库即可
$shim = Join-Path $cargoHome "gnu-shim"
if (-not (Test-Path "$shim\libgcc_eh.a")) {
    New-Item -ItemType Directory -Force $shim | Out-Null
    & "$w64\ar.exe" rc "$shim\libgcc_eh.a"
}
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-L native=$shim"
