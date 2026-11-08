//! 面板音效：直接播放游戏菜单自己的 Wwise 事件（光标移动 / 确定 / 返回），音色和音量设置都与游戏一致。
//! 叠加层只在渲染线程运行，用线程局部状态收集本帧的请求；帧末只播优先级最高的一个，避免一次操作叠出多个声音。

use std::cell::Cell;

use hudhook::imgui::{Ui, sys};

use crate::game::sound::{self, menu};

/// 按优先级从低到高：同一帧里既移动又确定时只播确定音，关闭面板时只播返回音
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cue {
    Cursor,
    Confirm,
    Cancel,
}

impl Cue {
    fn event(self) -> u32 {
        match self {
            Cue::Cursor => menu::CURSOR,
            Cue::Confirm => menu::CONFIRM,
            Cue::Cancel => menu::CANCEL,
        }
    }
}

thread_local! {
    static PENDING: Cell<Option<Cue>> = const { Cell::new(None) };
    /// 本帧被鼠标悬停的控件，以及上一次悬停过的控件（离开到空白处不清除，回到原控件不重复响）
    static HOVERED: Cell<u32> = const { Cell::new(0) };
    static LAST_HOVERED: Cell<u32> = const { Cell::new(0) };
}

pub fn request(cue: Cue) {
    PENDING.with(|p| p.set(p.get().max(Some(cue))));
}

/// 在可交互控件之后调用：鼠标停在它上面时记下它的 ID
pub fn track_hover(ui: &Ui) {
    if ui.is_item_hovered() {
        HOVERED.with(|h| h.set(unsafe { sys::igGetItemID() }));
    }
}

/// 每帧末尾调用：鼠标移到另一个控件上时补一个光标音，然后播放本帧的音效。
/// 只认鼠标移动造成的悬停变化，面板刚打开时控件出现在静止的指针下不响
pub fn end_frame(ui: &Ui, enabled: bool) {
    let hovered = HOVERED.with(|h| h.replace(0));
    let moved = ui.io().mouse_delta != [0.0, 0.0];
    if hovered != 0 && LAST_HOVERED.with(|l| l.replace(hovered)) != hovered && moved {
        request(Cue::Cursor);
    }
    if let Some(cue) = PENDING.with(Cell::take)
        && enabled
    {
        sound::post(cue.event());
    }
}
