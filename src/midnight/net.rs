//! 午夜状态的传输接口：开关、遗物检查、侧栏只依赖它，换通道只换实现（正式实现见 lobby.rs）。

use std::time::Instant;

use nr_stat_alloc_shared::midnight::wire::MemberState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Teammate {
    /// 侧栏序号（大厅成员顺序）
    pub slot: u8,
    pub name: Option<String>,
    /// None = 没有午夜状态（没装模组或版本不同）
    pub state: Option<MemberState>,
}

pub trait MidnightLink {
    /// 每帧调用（实现自行限频）
    fn poll(&mut self, now: Instant);
    /// Some(true) = 房主或单人；Some(false) = 队友；None = 身份未知
    fn is_host(&self) -> Option<bool>;
    fn publish(&mut self, own: &MemberState, now: Instant);
    fn teammates(&self) -> &[Teammate];
    /// 房主的状态（午夜开关与诅咒）；没有房主或房主没装模组时为 None
    fn host_state(&self) -> Option<MemberState>;
}

/// Steam 接口解析失败时用：身份未知，只在本机生效
pub struct NullLink;

impl MidnightLink for NullLink {
    fn poll(&mut self, _now: Instant) {}

    fn is_host(&self) -> Option<bool> {
        None
    }

    fn publish(&mut self, _own: &MemberState, _now: Instant) {}

    fn teammates(&self) -> &[Teammate] {
        &[]
    }

    fn host_state(&self) -> Option<MemberState> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_link_reports_unknown() {
        let mut l = NullLink;
        let t = Instant::now();
        l.poll(t);
        l.publish(&MemberState::default(), t);
        assert_eq!((l.is_host(), l.host_state(), l.teammates().len()), (None, None, 0));
    }
}
