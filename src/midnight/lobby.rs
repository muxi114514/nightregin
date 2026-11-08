//! 午夜联机的 Steam 大厅通道：每个玩家把自己的状态写进大厅成员数据（编码见 shared::midnight::wire），其他成员直接读；
//! 大厅主就是房主，房主的「在午夜队伍」即全队午夜开关。成员按 Steam ID 区分，不需要位置号。
//! 全部在每帧任务（游戏线程）里调用，只有进入大厅的回调来自 Steam（game::steam 里已隔离）。

use std::time::{Duration, Instant};

use log::info;
use nr_stat_alloc_shared::midnight::wire::{self, LOBBY_KEY, MAX_MEMBERS, MemberState};

use super::net::{MidnightLink, Teammate};
use crate::game::steam::SteamApi;

/// 读大厅的最短间隔
const POLL_EVERY: Duration = Duration::from_millis(250);
/// 记住最近进入过的几个大厅（无缝联机可能先后进多个）
const MAX_TRACKED: usize = 8;

/// LobbyLink 需要的大厅操作（正式实现转调 Steam，测试用假实现）
pub trait LobbyApi {
    /// 取走新进入的大厅（按进入顺序）
    fn take_entered(&self) -> Vec<u64>;
    fn self_id(&self) -> Option<u64>;
    fn owner(&self, lobby: u64) -> Option<u64>;
    /// 不在该大厅时为空
    fn members(&self, lobby: u64) -> Vec<u64>;
    fn member_data(&self, lobby: u64, user: u64) -> Option<String>;
    fn set_member_data(&self, lobby: u64, value: &str);
    fn name(&self, user: u64) -> Option<String>;
    /// 一轮读取结束
    fn end_poll(&self);
}

impl LobbyApi for SteamApi {
    fn take_entered(&self) -> Vec<u64> {
        SteamApi::take_entered(self)
    }

    fn self_id(&self) -> Option<u64> {
        SteamApi::self_id(self)
    }

    fn owner(&self, lobby: u64) -> Option<u64> {
        SteamApi::owner(self, lobby)
    }

    fn members(&self, lobby: u64) -> Vec<u64> {
        SteamApi::members(self, lobby)
    }

    fn member_data(&self, lobby: u64, user: u64) -> Option<String> {
        SteamApi::member_data(self, lobby, user, LOBBY_KEY)
    }

    fn set_member_data(&self, lobby: u64, value: &str) {
        SteamApi::set_member_data(self, lobby, LOBBY_KEY, value);
    }

    fn name(&self, user: u64) -> Option<String> {
        self.persona_name(user)
    }

    fn end_poll(&self) {
        self.release_thread_memory();
    }
}

#[derive(Default)]
struct LobbyView {
    is_host: Option<bool>,
    lobby: Option<u64>,
    owner: Option<u64>,
    teammates: Vec<Teammate>,
    host_state: Option<MemberState>,
    /// host_state 是从上一个大厅沿用的（新大厅的数据还没到）
    carried: bool,
}

pub struct LobbyLink<A> {
    api: A,
    resend: Duration,
    last_poll: Option<Instant>,
    /// 进入过的大厅，最近的在前
    entered: Vec<u64>,
    view: LobbyView,
    last_state: Option<MemberState>,
    last_sent: Option<Instant>,
    logged: Option<String>,
}

impl<A: LobbyApi> LobbyLink<A> {
    pub fn new(api: A, resend: Duration) -> Self {
        Self { api, resend, last_poll: None, entered: Vec::new(), view: LobbyView::default(), last_state: None, last_sent: None, logged: None }
    }

    fn read_view(&self) -> LobbyView {
        let Some(me) = self.api.self_id() else { return LobbyView::default() };
        let current = self.entered.iter().find_map(|&id| {
            let members = self.api.members(id);
            members.contains(&me).then_some((id, members))
        });
        let Some((lobby, members)) = current else {
            return LobbyView { is_host: Some(true), ..Default::default() };
        };
        let state_of = |user| self.api.member_data(lobby, user).and_then(|t| wire::decode(&t));
        let owner = self.api.owner(lobby);
        let teammates = members
            .iter()
            .filter(|&&m| m != me)
            .take(MAX_MEMBERS as usize - 1)
            .enumerate()
            .map(|(i, &m)| Teammate { slot: i as u8, name: self.api.name(m), state: state_of(m) })
            .collect();
        LobbyView { is_host: owner.map(|o| o == me), lobby: Some(lobby), owner, teammates, host_state: owner.filter(|&o| o != me).and_then(state_of), carried: false }
    }

    /// 出击时换到出征大厅，房主的数据要晚一点才到：同一个房主先沿用上次的状态，午夜和诅咒不闪断
    fn carry_host_state(&self, view: &mut LobbyView) {
        let waiting = view.lobby != self.view.lobby || self.view.carried;
        if view.host_state.is_none() && waiting && view.owner.is_some() && view.owner == self.view.owner {
            view.host_state = self.view.host_state;
            view.carried = view.host_state.is_some();
        }
    }

    /// 超上限时优先丢已经不在里面的大厅：回圆桌不会再收到进入事件，最老的房间大厅不能丢
    fn trim_entered(&mut self) {
        while self.entered.len() > MAX_TRACKED {
            let left = self.api.self_id().and_then(|me| self.entered.iter().rposition(|&id| !self.api.members(id).contains(&me)));
            self.entered.remove(left.unwrap_or(self.entered.len() - 1));
        }
    }

    /// 大厅或队友状态变了才写日志（联机排查用，ID 只记尾数）
    fn log_changes(&mut self) {
        let states: Vec<_> = self.view.teammates.iter().map(|t| t.state.as_ref().map(wire::encode)).collect();
        let line = self.view.lobby.map(|id| format!("lobby ..{} host {:?} teammates {states:?}", id % 1_000_000, self.view.is_host));
        if line != self.logged {
            info!("midnight: {}", line.as_deref().unwrap_or("not in a lobby"));
            self.logged = line;
        }
    }
}

impl<A: LobbyApi> MidnightLink for LobbyLink<A> {
    fn poll(&mut self, now: Instant) {
        if self.last_poll.is_some_and(|t| now.saturating_duration_since(t) < POLL_EVERY) {
            return;
        }
        self.last_poll = Some(now);
        for id in self.api.take_entered() {
            self.entered.retain(|&l| l != id);
            self.entered.insert(0, id);
        }
        self.trim_entered();
        let mut view = self.read_view();
        self.carry_host_state(&mut view);
        // 换了大厅，新大厅里还没有自己的状态，马上写
        if view.lobby != self.view.lobby {
            self.last_sent = None;
        }
        self.view = view;
        self.log_changes();
        self.api.end_poll();
    }

    fn is_host(&self) -> Option<bool> {
        self.view.is_host
    }

    fn publish(&mut self, own: &MemberState, now: Instant) {
        let Some(lobby) = self.view.lobby else { return };
        let changed = self.last_state != Some(*own);
        if !changed && self.last_sent.is_some_and(|t| now.saturating_duration_since(t) < self.resend) {
            return;
        }
        let value = wire::encode(own);
        self.api.set_member_data(lobby, &value);
        if changed {
            info!("midnight: published state {value}");
        }
        self.last_state = Some(*own);
        self.last_sent = Some(now);
    }

    fn teammates(&self) -> &[Teammate] {
        &self.view.teammates
    }

    fn host_state(&self) -> Option<MemberState> {
        self.view.host_state
    }
}

#[cfg(test)]
#[path = "lobby_tests.rs"]
mod tests;
