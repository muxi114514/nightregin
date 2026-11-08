//! 锻造台对话脚本的「继承词条」选项：脚本执行 SetEventFlag(REQUEST_FLAG, 1)，旗标分发器（game::event_flags）把它交给这里截下，由每帧任务取走。
//! 脚本先写本地旗标（请求同步），再把旗标放进发给队友的同步队列；两处都截下 REQUEST_FLAG：块 1996800 不存在，也不能让队友跟着打开面板。
//! 队友发来的这个旗标（对方是旧版或没钩上同步队列）以「不请求同步」写入，据此忽略，只认本机脚本。

use std::sync::atomic::{AtomicBool, Ordering};

use log::info;

use crate::game::event_flags::FlagVerdict;

/// 对话脚本里写的旗标编号（块 1996800 不存在）
pub const REQUEST_FLAG: u32 = 1_996_800_001;

static REQUESTED: AtomicBool = AtomicBool::new(false);

/// 请求打开继承面板（对话脚本之外的入口，如蓝10）
pub fn request() {
    REQUESTED.store(true, Ordering::Release);
}

/// 自上次调用以来本机是否请求过继承
pub fn take_request() -> bool {
    REQUESTED.swap(false, Ordering::AcqRel)
}

/// set_event_flag 的处理函数
pub fn on_flag(id: u32, value: u8, sync: u8) -> FlagVerdict {
    if id != REQUEST_FLAG {
        return FlagVerdict::Pass;
    }
    if value != 0 && sync != 0 {
        REQUESTED.store(true, Ordering::Release);
    } else if value != 0 {
        info!("passive inherit: ignored a request synced from a teammate");
    }
    FlagVerdict::Swallow
}

/// 同步队列过滤：继承请求不发给队友
pub fn block_queue(id: u32) -> bool {
    id == REQUEST_FLAG
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 共用全局请求位，放在一个测试里顺序执行，避免并行测试互相干扰
    #[test]
    fn request_only_from_local_script() {
        let _ = take_request();
        assert_eq!(on_flag(REQUEST_FLAG, 1, 0), FlagVerdict::Swallow);
        assert!(!take_request());
        assert_eq!(on_flag(REQUEST_FLAG, 1, 1), FlagVerdict::Swallow);
        assert!(take_request());
    }

    #[test]
    fn other_flags_pass() {
        assert_eq!(on_flag(151, 1, 1), FlagVerdict::Pass);
        assert!(block_queue(REQUEST_FLAG));
        assert!(!block_queue(151));
    }
}
