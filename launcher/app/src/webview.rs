//! WebView2：关掉默认右键菜单和浏览器快捷键；渲染进程崩溃时自动重载页面

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tauri::WebviewWindow;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
    ICoreWebView2Controller, ICoreWebView2Settings3,
};
use webview2_com::ProcessFailedEventHandler;
use windows::core::Interface;

use crate::panic_dialog;

const MAX_RELOADS: usize = 3;
const RELOAD_WINDOW: Duration = Duration::from_secs(60);

pub fn setup(window: &WebviewWindow) {
    let result = window.with_webview(|webview| {
        let controller = webview.controller();
        match apply_settings(&controller) {
            Ok(()) => log::info!("已关闭 WebView2 默认右键菜单和浏览器快捷键"),
            Err(e) => log::error!("关闭 WebView2 右键菜单和浏览器快捷键失败：{e}"),
        }
        if let Err(e) = watch_process_failed(&controller) {
            log::error!("监听 WebView2 进程崩溃失败：{e}");
        }
    });
    if let Err(e) = result {
        log::error!("取不到 WebView2：{e}");
    }
}

fn apply_settings(controller: &ICoreWebView2Controller) -> windows::core::Result<()> {
    // SAFETY: controller 在 with_webview 回调期间有效，接口都由它取得
    unsafe {
        let settings = controller.CoreWebView2()?.Settings()?;
        settings.SetAreDefaultContextMenusEnabled(false)?;
        settings
            .cast::<ICoreWebView2Settings3>()?
            .SetAreBrowserAcceleratorKeysEnabled(false)?;
    }
    Ok(())
}

fn watch_process_failed(controller: &ICoreWebView2Controller) -> windows::core::Result<()> {
    let limiter = Arc::new(Mutex::new(ReloadLimiter::default()));
    let handler = ProcessFailedEventHandler::create(Box::new(move |sender, args| {
        let (Some(webview), Some(args)) = (sender, args) else {
            return Ok(());
        };
        let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
        // SAFETY: args 在回调期间有效，kind 可写
        unsafe { args.ProcessFailedKind(&mut kind)? };
        log::error!("WebView2 进程崩溃，类型 {}", kind.0);
        if !should_reload(kind) {
            return Ok(());
        }
        let allowed = limiter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .allow(Instant::now());
        if allowed {
            // SAFETY: webview 在回调期间有效
            unsafe { webview.Reload()? };
        } else {
            // 这里是界面线程，弹框放到别的线程，免得卡住窗口消息
            let spawned = std::thread::Builder::new()
                .name("render-crash-report".into())
                .spawn(|| panic_dialog::offer("启动器页面反复崩溃。"));
            if let Err(e) = spawned {
                log::error!("无法创建线程：{e}");
            }
        }
        Ok(())
    }));
    let mut token = 0;
    // SAFETY: controller 有效；handler 由 WebView2 持有引用，token 可写
    unsafe {
        controller
            .CoreWebView2()?
            .add_ProcessFailed(&handler, &mut token)
    }
}

fn should_reload(kind: COREWEBVIEW2_PROCESS_FAILED_KIND) -> bool {
    kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
}

/// 60 秒内最多重载 3 次，免得页面一打开就崩时无限重载
#[derive(Default)]
struct ReloadLimiter {
    recent: VecDeque<Instant>,
}

impl ReloadLimiter {
    fn allow(&mut self, now: Instant) -> bool {
        while self
            .recent
            .front()
            .is_some_and(|t| now.saturating_duration_since(*t) >= RELOAD_WINDOW)
        {
            self.recent.pop_front();
        }
        if self.recent.len() >= MAX_RELOADS {
            return false;
        }
        self.recent.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE,
    };

    use super::{ReloadLimiter, should_reload};

    #[test]
    fn should_reload_only_render_exit() {
        assert!(should_reload(
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
        ));
        assert!(!should_reload(
            COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED
        ));
        assert!(!should_reload(
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE
        ));
    }

    #[test]
    fn reload_limiter_three_per_minute() {
        let t0 = Instant::now();
        let mut limiter = ReloadLimiter::default();
        for s in 0..3 {
            assert!(limiter.allow(t0 + Duration::from_secs(s)));
        }
        assert!(!limiter.allow(t0 + Duration::from_secs(10)));
        // 第一次重载已过 60 秒，又腾出一次
        assert!(limiter.allow(t0 + Duration::from_secs(61)));
    }
}
