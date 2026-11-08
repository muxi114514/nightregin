//! 桌面快捷方式：启动器在游戏目录下三层，不好找

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize, IPersistFile,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::core::{HSTRING, Interface};

/// 目标 = 当前 exe，工作目录 = exe 目录，图标 = exe；已存在就覆盖
pub fn create_desktop_shortcut(desktop: &Path, title: &str) -> Result<PathBuf, String> {
    let target = std::env::current_exe().map_err(|e| format!("取不到启动器路径：{e}"))?;
    let workdir = target
        .parent()
        .ok_or_else(|| "取不到启动器目录".to_owned())?
        .to_path_buf();
    let lnk = desktop.join(format!("{title}.lnk"));
    create_shortcut_at(&lnk, &target, &workdir).map_err(|e| format!("建快捷方式失败：{e}"))?;
    Ok(lnk)
}

/// 线程上的 COM 初始化；CoInitializeEx 成功（含 S_FALSE）才配对释放
struct ComGuard(bool);

impl ComGuard {
    fn new() -> ComGuard {
        // SAFETY: 第二个参数是合法的并发模型
        ComGuard(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok())
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: 与本线程上成功的 CoInitializeEx 配对
            unsafe { CoUninitialize() };
        }
    }
}

fn create_shortcut_at(lnk: &Path, target: &Path, workdir: &Path) -> windows::core::Result<()> {
    let _com = ComGuard::new();
    // SAFETY: 接口都由 CoCreateInstance / cast 取得，字符串在调用期间有效
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from(target))?;
        link.SetWorkingDirectory(&HSTRING::from(workdir))?;
        link.SetIconLocation(&HSTRING::from(target), 0)?;
        let file: IPersistFile = link.cast()?;
        file.Save(&HSTRING::from(lnk), true)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface};

    use super::{ComGuard, create_shortcut_at};

    fn utf16(buf: &[u16]) -> PathBuf {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        PathBuf::from(String::from_utf16_lossy(&buf[..len]))
    }

    fn read_back(lnk: &Path) -> (PathBuf, PathBuf) {
        let _com = ComGuard::new();
        // SAFETY: 同 create_shortcut_at；缓冲区按长度传入
        unsafe {
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            let file: IPersistFile = link.cast().unwrap();
            file.Load(&HSTRING::from(lnk), STGM_READ).unwrap();
            let mut path = [0u16; 520];
            link.GetPath(&mut path, std::ptr::null_mut(), 0).unwrap();
            let mut dir = [0u16; 520];
            link.GetWorkingDirectory(&mut dir).unwrap();
            (utf16(&path), utf16(&dir))
        }
    }

    #[test]
    fn shortcut_points_at_exe() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("黑夜 君临");
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("启动器.exe");
        std::fs::write(&target, b"x").unwrap();
        let lnk = tmp.path().join("黑夜君临黄金时代.lnk");
        create_shortcut_at(&lnk, &target, &dir).unwrap();
        // 临时目录可能是 8.3 短路径，系统读回来是长路径，只比后面几段
        let (got_target, got_dir) = read_back(&lnk);
        assert!(
            got_target.ends_with(Path::new("黑夜 君临").join("启动器.exe")),
            "{got_target:?}"
        );
        assert!(got_dir.ends_with("黑夜 君临"), "{got_dir:?}");
    }
}
