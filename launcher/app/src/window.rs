//! 主窗口：尺寸与位置、显示与隐藏、关掉 WebView2 的浏览器功能、可见性与关窗口事件

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use launcher_core::launch::LaunchView;
use serde::Serialize;
use tauri::window::Color;
use tauri::{
    AppHandle, CloseRequestApi, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize,
    Size, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent,
};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DwmSetWindowAttribute,
};

use crate::commands::{self, CLOSE_REQUESTED, WINDOW_VISIBILITY};
use crate::state::{self, AppState};
use crate::tray;

/// 主窗口的 label
pub const MAIN: &str = "main";
/// 设计尺寸（逻辑像素）
pub const DESIGN_W: f64 = 1200.0;
pub const DESIGN_H: f64 = 720.0;
/// 窗口最多占工作区的比例
pub const MAX_RATIO: f64 = 0.92;
/// 前端迟迟不调 ui_ready 时，过这么久也显示窗口
const REVEAL_FALLBACK: Duration = Duration::from_secs(3);
/// Win11 起的 build 号：Win10 上无边框窗口开阴影会多一圈 1px 白边
const WIN11_BUILD: u32 = 22000;

/// 首次显示做过没有：ui_ready 和 3 秒兜底谁先到谁做
static REVEALED: AtomicBool = AtomicBool::new(false);
/// 上次发给前端的可见状态；只在主线程上读写
static VISIBLE: AtomicBool = AtomicBool::new(false);

/// 物理像素的矩形
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Serialize)]
struct Visibility {
    visible: bool,
}

/// 设计尺寸 × 缩放得物理尺寸，只缩不放：系数 = min(1, 工作区宽 × 0.92 / 物理宽, 工作区高 × 0.92 / 物理高)；
/// 宽高四舍五入，在工作区里居中（向下取整）
pub fn fit_window(work: Rect, scale: f64) -> Rect {
    let (w, h) = (DESIGN_W * scale, DESIGN_H * scale);
    let k = 1f64
        .min(f64::from(work.w) * MAX_RATIO / w)
        .min(f64::from(work.h) * MAX_RATIO / h);
    let (w, h) = ((w * k).round() as u32, (h * k).round() as u32);
    Rect {
        x: centered(work.x, work.w, w),
        y: centered(work.y, work.h, h),
        w,
        h,
    }
}

/// start + (span − len) / 2，向下取整
fn centered(start: i32, span: u32, len: u32) -> i32 {
    let offset = (i64::from(span) - i64::from(len)).div_euclid(2);
    i32::try_from(i64::from(start) + offset).unwrap_or(start)
}

/// 建主窗口：隐藏、无边框、黑底（和 html / body 一致，不白闪）、关掉缩放快捷键。
/// 系统层面设成可缩放：开阴影时 tao 在左右下留的那圈边框才会是透明的调整边框，不可缩放时会被画成白边；
/// 实际尺寸由 place() 用最小 = 最大锁死。不能靠改 DWM 框架去白边，那样 RTSS 会注入并弄崩 WebView2
pub fn build_main(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let win11 = windows_version::OsVersion::current().build >= WIN11_BUILD;
    let window = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html".into()))
        .title(&app.package_info().name)
        .inner_size(DESIGN_W, DESIGN_H)
        .visible(false)
        .decorations(false)
        .resizable(true)
        .maximizable(false)
        .shadow(win11)
        .background_color(Color(0, 0, 0, 255))
        .zoom_hotkeys_enabled(false)
        .build()?;
    if win11 {
        hide_system_border(&window);
    }
    Ok(window)
}

/// Win11 给开阴影的无边框窗口加一圈 1px 系统边框，失焦时是浅色：设成不画（阴影保留）。失败只记日志
fn hide_system_border(window: &WebviewWindow) {
    let hwnd = match window.hwnd() {
        Ok(hwnd) => hwnd.0,
        Err(e) => {
            log::warn!("取不到主窗口句柄，系统边框保留：{e}");
            return;
        }
    };
    let color: u32 = DWMWA_COLOR_NONE;
    // SAFETY: hwnd 是刚建好的主窗口；color 是 4 字节的 COLORREF，调用期间有效
    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            (&raw const color).cast(),
            size_of::<u32>() as u32,
        )
    };
    if hr < 0 {
        log::warn!("去掉主窗口的系统边框失败（HRESULT {hr:#010x}）");
    } else {
        log::info!("已去掉主窗口的系统边框（DWMWA_BORDER_COLOR = DWMWA_COLOR_NONE）");
    }
}

/// 前端 3 秒内没调 ui_ready 也显示窗口，免得一直隐藏
pub fn spawn_reveal_fallback(app: &AppHandle) {
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("reveal-fallback".into())
        .spawn(move || {
            std::thread::sleep(REVEAL_FALLBACK);
            if reveal(&app) {
                log::warn!(
                    "前端 {} 秒内没有就绪，已先显示窗口",
                    REVEAL_FALLBACK.as_secs()
                );
            }
        });
    if let Err(e) = spawned {
        log::error!("无法创建兜底显示线程：{e}");
    }
}

/// 首次显示，只做一次；这次真的显示了才返回 true
pub fn reveal(app: &AppHandle) -> bool {
    let first = !REVEALED.swap(true, Ordering::SeqCst);
    if first {
        show_main(app);
    }
    first
}

/// 还原、显示并聚焦主窗口；隐藏着时（首次显示、从托盘回来）先按鼠标所在显示器定尺寸和位置
pub fn show_main(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };
    REVEALED.store(true, Ordering::SeqCst);
    let hidden = !window.is_visible().unwrap_or(false);
    check("还原主窗口", window.unminimize());
    if hidden {
        place(&window);
    }
    check("显示主窗口", window.show());
    check("聚焦主窗口", window.set_focus());
    sync_visibility(app);
}

/// 游戏运行时隐入托盘；托盘没建成就改为最小化，免得窗口找不回来
pub fn hide_to_tray(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };
    if tray::exists(app) {
        check("隐藏主窗口", window.hide());
    } else {
        check("最小化主窗口", window.minimize());
    }
    sync_visibility(app);
}

/// 启动以终态结束：Finished 总把窗口显示回来；Failed / Cancelled 只在窗口隐藏着（例如隐入了托盘）时显示，
/// 玩家自己最小化的保持原样
pub fn show_after_end(app: &AppHandle, view: &LaunchView) {
    let visible = app
        .get_webview_window(MAIN)
        .is_some_and(|w| w.is_visible().unwrap_or(false));
    if reshow_after_end(view, visible) {
        show_main(app);
    } else {
        sync_visibility(app);
    }
}

/// visible 取 IsWindowVisible：最小化的窗口也算显示着
fn reshow_after_end(view: &LaunchView, visible: bool) -> bool {
    match view {
        LaunchView::Finished { .. } => true,
        LaunchView::Failed { .. } | LaunchView::Cancelled => !visible,
        _ => false,
    }
}

pub fn minimize_main(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        check("最小化主窗口", window.minimize());
    }
    sync_visibility(app);
}

/// 冒烟退出前先关窗口：窗口还在时退出，WebView2 会往 stderr 打「Failed to unregister class」
pub fn destroy_main(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        check("关闭主窗口", window.destroy());
    }
}

/// 前端据此暂停 / 恢复动画：窗口显示着、没有最小化、游戏没在运行才算可见，变了才发 window-visibility。
/// 一律放到主线程上判断和发送，各处调用的先后就是前端收到的先后
pub fn sync_visibility(app: &AppHandle) {
    let handle = app.clone();
    if let Err(e) = app.run_on_main_thread(move || update_visibility(&handle)) {
        log::warn!("同步窗口可见状态失败：{e}");
    }
}

fn update_visibility(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    let shown = window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false);
    let game_running = app
        .try_state::<AppState>()
        .is_some_and(|state| state.game_running());
    let visible = shown && !game_running;
    if VISIBLE.swap(visible, Ordering::SeqCst) != visible {
        commands::emit(app, WINDOW_VISIBILITY, Visibility { visible });
    }
}

/// 最小化、还原都会触发 Resized；关窗口（Alt+F4 等）见 on_close_requested
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { api, .. } => on_close_requested(window.app_handle(), api),
        WindowEvent::Resized(_) => sync_visibility(window.app_handle()),
        WindowEvent::Focused(true) => {
            commands::game_settings::recheck_on_focus(window.app_handle())
        }
        _ => {}
    }
}

/// 启动进行中（含唤起中）：拦下并交前端确认，和 × 一样。
/// 否则保存设置后退出，窗口照常关掉（先于退出销毁，WebView2 不会往 stderr 报 Failed to unregister class）
fn on_close_requested(app: &AppHandle, api: &CloseRequestApi) {
    let confirm = app
        .try_state::<AppState>()
        .is_some_and(|state| state.launch_in_progress());
    if confirm {
        api.prevent_close();
        commands::emit(app, CLOSE_REQUESTED, ());
    } else {
        state::quit(app);
    }
}

/// 让内容区落在 fit_window 算出的位置和尺寸上。
/// 先移到目标显示器（跨缩放比例时系统会按新比例改尺寸），再设尺寸，最后按外框与内容区之差复位
fn place(window: &WebviewWindow) {
    let Some(monitor) = target_monitor(window) else {
        log::warn!("取不到显示器信息，主窗口沿用当前尺寸和位置");
        return;
    };
    let area = monitor.work_area();
    let work = Rect {
        x: area.position.x,
        y: area.position.y,
        w: area.size.width,
        h: area.size.height,
    };
    let fit = fit_window(work, monitor.scale_factor());
    // 先放开尺寸锁，免得跨显示器重排时被上一次的锁卡住
    check("放开主窗口尺寸锁", window.set_min_size(None::<Size>));
    check("放开主窗口尺寸锁", window.set_max_size(None::<Size>));
    check(
        "移动主窗口",
        window.set_position(PhysicalPosition::new(fit.x, fit.y)),
    );
    check(
        "设置主窗口尺寸",
        window.set_size(PhysicalSize::new(fit.w, fit.h)),
    );
    // 最小 = 最大 = 当前尺寸：系统层面可缩放（去白边），实际拖不动；用逻辑尺寸，拖到别的缩放比例的显示器也成立
    let scale = monitor.scale_factor();
    let fixed = LogicalSize::new(f64::from(fit.w) / scale, f64::from(fit.h) / scale);
    check("锁定主窗口尺寸", window.set_min_size(Some(fixed)));
    check("锁定主窗口尺寸", window.set_max_size(Some(fixed)));
    let (dx, dy) = frame_offset(window);
    let outer = PhysicalPosition::new(fit.x.saturating_sub(dx), fit.y.saturating_sub(dy));
    check("移动主窗口", window.set_position(outer));
}

/// 内容区左上角相对外框左上角的偏移：set_position 定的是外框，
/// Win11 开阴影的无边框窗口外框还含左右下各一圈不可见的边和顶上 1px；取不到时当作 0
fn frame_offset(window: &WebviewWindow) -> (i32, i32) {
    match (window.inner_position(), window.outer_position()) {
        (Ok(inner), Ok(outer)) => (inner.x - outer.x, inner.y - outer.y),
        _ => (0, 0),
    }
}

/// 鼠标所在的显示器；取不到时用窗口当前所在的，再不行用主显示器
fn target_monitor(window: &WebviewWindow) -> Option<Monitor> {
    let at_cursor = window
        .cursor_position()
        .ok()
        .and_then(|cursor| window.monitor_from_point(cursor.x, cursor.y).ok().flatten());
    at_cursor
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten())
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    let window = app.get_webview_window(MAIN);
    if window.is_none() {
        log::error!("找不到主窗口");
    }
    window
}

/// 窗口操作失败只记日志
fn check(what: &str, result: tauri::Result<()>) {
    if let Err(e) = result {
        log::warn!("{what}失败：{e}");
    }
}

#[cfg(test)]
mod tests {
    use launcher_core::launch::{FailReason, LaunchView, Step};

    use super::{Rect, fit_window, reshow_after_end};

    /// LC-3：失败或取消时，隐入托盘的窗口显示回来；显示着或被最小化的保持原样；Finished 总显示
    #[test]
    fn failed_or_cancelled_reshow_only_hidden_window() {
        let failed = LaunchView::Failed {
            step: Step::Wake,
            reason: FailReason::GameNeverStarted,
        };
        for view in [failed, LaunchView::Cancelled] {
            assert!(reshow_after_end(&view, false), "{view:?}");
            assert!(!reshow_after_end(&view, true), "{view:?}");
        }
        assert!(reshow_after_end(
            &LaunchView::Finished { abnormal: false },
            true
        ));
        assert!(reshow_after_end(
            &LaunchView::Finished { abnormal: true },
            false
        ));
    }

    #[test]
    fn fit_window_large_screen() {
        let work = Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1040,
        };
        assert_eq!(
            fit_window(work, 1.0),
            Rect {
                x: 360,
                y: 160,
                w: 1200,
                h: 720
            }
        );
    }

    #[test]
    fn fit_window_small_screen_125() {
        let work = Rect {
            x: 0,
            y: 0,
            w: 1366,
            h: 728,
        };
        assert_eq!(
            fit_window(work, 1.25),
            Rect {
                x: 125,
                y: 29,
                w: 1116,
                h: 670
            }
        );
    }

    #[test]
    fn fit_window_secondary_monitor() {
        let work = Rect {
            x: 1920,
            y: 0,
            w: 2560,
            h: 1400,
        };
        assert_eq!(
            fit_window(work, 1.0),
            Rect {
                x: 2600,
                y: 340,
                w: 1200,
                h: 720
            }
        );
    }

    /// 不受工作区限制时按缩放放大（本机 1920×1080、125% 实测同此）
    #[test]
    fn fit_window_scales_with_dpi() {
        let work = Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1020,
        };
        assert_eq!(
            fit_window(work, 1.25),
            Rect {
                x: 210,
                y: 60,
                w: 1500,
                h: 900
            }
        );
    }

    /// 竖长的工作区由宽度限制：1280 × 0.92 / 1200
    #[test]
    fn fit_window_narrow_screen_limits_width() {
        let work = Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 1400,
        };
        assert_eq!(
            fit_window(work, 1.0),
            Rect {
                x: 51,
                y: 346,
                w: 1178,
                h: 707
            }
        );
    }

    /// 剩余空间是奇数时位置向下取整
    #[test]
    fn fit_window_odd_margin_rounds_down() {
        let work = Rect {
            x: -1921,
            y: 0,
            w: 1921,
            h: 1041,
        };
        assert_eq!(
            fit_window(work, 1.0),
            Rect {
                x: -1561,
                y: 160,
                w: 1200,
                h: 720
            }
        );
    }
}
