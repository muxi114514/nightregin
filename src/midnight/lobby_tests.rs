//! LobbyLink 的测试：用假的大厅接口驱动。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use super::*;

const ME: u64 = 100;
const HOST: u64 = 200;
const MATE: u64 = 300;
const ROOM: u64 = 9000;
const OTHER_ROOM: u64 = 9001;

struct Fake {
    entered: RefCell<Vec<u64>>,
    me: Cell<Option<u64>>,
    owners: RefCell<HashMap<u64, u64>>,
    members: RefCell<HashMap<u64, Vec<u64>>>,
    data: RefCell<HashMap<(u64, u64), String>>,
    names: RefCell<HashMap<u64, String>>,
    sets: Cell<usize>,
    end_polls: Cell<usize>,
}

impl Fake {
    fn new() -> Self {
        Self {
            entered: RefCell::default(),
            me: Cell::new(Some(ME)),
            owners: RefCell::default(),
            members: RefCell::default(),
            data: RefCell::default(),
            names: RefCell::default(),
            sets: Cell::new(0),
            end_polls: Cell::new(0),
        }
    }

    fn enter(&self, lobby: u64, owner: u64, members: &[u64]) {
        self.entered.borrow_mut().push(lobby);
        self.owners.borrow_mut().insert(lobby, owner);
        self.members.borrow_mut().insert(lobby, members.to_vec());
    }

    fn leave(&self, lobby: u64) {
        self.members.borrow_mut().remove(&lobby);
        self.owners.borrow_mut().remove(&lobby);
    }

    fn write(&self, lobby: u64, user: u64, text: &str) {
        self.data.borrow_mut().insert((lobby, user), text.to_string());
    }

    fn own_data(&self, lobby: u64) -> Option<String> {
        self.data.borrow().get(&(lobby, ME)).cloned()
    }
}

impl LobbyApi for &Fake {
    fn take_entered(&self) -> Vec<u64> {
        std::mem::take(&mut *self.entered.borrow_mut())
    }

    fn self_id(&self) -> Option<u64> {
        self.me.get()
    }

    fn owner(&self, lobby: u64) -> Option<u64> {
        self.owners.borrow().get(&lobby).copied()
    }

    fn members(&self, lobby: u64) -> Vec<u64> {
        self.members.borrow().get(&lobby).cloned().unwrap_or_default()
    }

    fn member_data(&self, lobby: u64, user: u64) -> Option<String> {
        self.data.borrow().get(&(lobby, user)).cloned()
    }

    fn set_member_data(&self, lobby: u64, value: &str) {
        self.sets.set(self.sets.get() + 1);
        self.data.borrow_mut().insert((lobby, ME), value.to_string());
    }

    fn name(&self, user: u64) -> Option<String> {
        self.names.borrow().get(&user).cloned()
    }

    fn end_poll(&self) {
        self.end_polls.set(self.end_polls.get() + 1);
    }
}

fn link(fake: &Fake) -> LobbyLink<&Fake> {
    LobbyLink::new(fake, Duration::from_secs(5))
}

fn polled(fake: &Fake) -> (LobbyLink<&Fake>, Instant) {
    let mut l = link(fake);
    let t = Instant::now();
    l.poll(t);
    (l, t)
}

#[test]
fn without_lobby_is_solo_host() {
    let fake = Fake::new();
    let (l, _) = polled(&fake);
    assert_eq!(l.is_host(), Some(true));
    assert!(l.teammates().is_empty());
    assert_eq!(l.host_state(), None);
}

#[test]
fn unknown_steam_user_is_unknown_identity() {
    let fake = Fake::new();
    fake.me.set(None);
    assert_eq!(polled(&fake).0.is_host(), None);
}

#[test]
fn lobby_owner_is_host() {
    let fake = Fake::new();
    fake.enter(ROOM, ME, &[ME, MATE]);
    assert_eq!(polled(&fake).0.is_host(), Some(true));
}

#[test]
fn non_owner_is_guest() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    assert_eq!(polled(&fake).0.is_host(), Some(false));
}

#[test]
fn unknown_owner_is_unknown_identity() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    fake.owners.borrow_mut().clear();
    assert_eq!(polled(&fake).0.is_host(), None);
}

#[test]
fn host_state_comes_from_owner_data() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME, MATE]);
    fake.write(ROOM, HOST, "1,0,0,3,9");
    fake.write(ROOM, MATE, "0,0,0,9,2");
    let (mut l, t) = polled(&fake);
    assert_eq!(l.host_state().map(|s| (s.in_midnight, s.curse)), Some((true, 9)));
    fake.write(ROOM, HOST, "0,0,0,3,4");
    l.poll(t + POLL_EVERY);
    assert_eq!(l.host_state().map(|s| (s.in_midnight, s.curse)), Some((false, 4)));
}

#[test]
fn owner_without_mod_gives_no_mode() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    assert_eq!(polled(&fake).0.host_state(), None);
}

#[test]
fn host_ignores_own_data_for_mode() {
    let fake = Fake::new();
    fake.enter(ROOM, ME, &[ME, MATE]);
    fake.write(ROOM, ME, "1,0,0,3,0");
    assert_eq!(polled(&fake).0.host_state(), None);
}

#[test]
fn teammates_exclude_self_in_member_order() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME, MATE]);
    fake.write(ROOM, HOST, "1,0,1,3,0");
    fake.names.borrow_mut().insert(HOST, "Melina".to_string());
    let (l, _) = polled(&fake);
    assert_eq!(
        l.teammates(),
        [
            Teammate { slot: 0, name: Some("Melina".to_string()), state: Some(MemberState { in_midnight: true, has_relics: false, ready: true, level: 3, curse: 0 }) },
            Teammate { slot: 1, name: None, state: None },
        ]
    );
}

#[test]
fn malformed_member_data_counts_as_no_mod() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    fake.write(ROOM, HOST, "garbage");
    assert_eq!(polled(&fake).0.teammates()[0].state, None);
}

#[test]
fn teammates_are_capped_at_party_size() {
    let fake = Fake::new();
    let members: Vec<u64> = (0..20).map(|i| if i == 3 { ME } else { 1000 + i }).collect();
    fake.enter(ROOM, members[0], &members);
    assert_eq!(polled(&fake).0.teammates().len(), MAX_MEMBERS as usize - 1);
}

#[test]
fn leaving_the_lobby_returns_to_solo() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    let (mut l, t) = polled(&fake);
    assert_eq!(l.is_host(), Some(false));
    fake.leave(ROOM);
    l.poll(t + POLL_EVERY);
    assert_eq!(l.is_host(), Some(true));
    assert!(l.teammates().is_empty());
}

#[test]
fn most_recent_lobby_we_are_in_wins() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    fake.enter(OTHER_ROOM, ME, &[ME]);
    let (mut l, t) = polled(&fake);
    assert_eq!(l.is_host(), Some(true));
    // 退出后一个大厅，回到前一个
    fake.leave(OTHER_ROOM);
    l.poll(t + POLL_EVERY);
    assert_eq!(l.is_host(), Some(false));
}

#[test]
fn reentering_a_lobby_moves_it_to_front() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    fake.enter(OTHER_ROOM, ME, &[ME]);
    let (mut l, t) = polled(&fake);
    fake.entered.borrow_mut().push(ROOM);
    l.poll(t + POLL_EVERY);
    assert_eq!(l.is_host(), Some(false));
}

/// 回圆桌不会再收到进入事件，打很多局后房间大厅也不能被挤出跟踪列表
#[test]
fn room_lobby_survives_many_expeditions() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    let (mut l, mut t) = polled(&fake);
    for i in 0..10 {
        let expedition = 20_000 + i;
        fake.enter(expedition, HOST, &[HOST, ME]);
        t += POLL_EVERY;
        l.poll(t);
        fake.leave(expedition);
        t += POLL_EVERY;
        l.poll(t);
    }
    assert_eq!((l.view.lobby, l.is_host()), (Some(ROOM), Some(false)));
}

#[test]
fn tracked_lobbies_are_bounded() {
    let fake = Fake::new();
    for id in 0..50 {
        fake.entered.borrow_mut().push(id);
    }
    assert!(polled(&fake).0.entered.len() <= MAX_TRACKED);
}

#[test]
fn polling_is_throttled() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    let (mut l, t) = polled(&fake);
    fake.enter(OTHER_ROOM, ME, &[ME]);
    l.poll(t + Duration::from_millis(100));
    assert_eq!(l.is_host(), Some(false));
    assert_eq!(fake.end_polls.get(), 1);
    l.poll(t + POLL_EVERY);
    assert_eq!(l.is_host(), Some(true));
    assert_eq!(fake.end_polls.get(), 2);
}

#[test]
fn solo_publishes_nothing() {
    let fake = Fake::new();
    let (mut l, t) = polled(&fake);
    l.publish(&MemberState::default(), t);
    assert_eq!(fake.sets.get(), 0);
}

#[test]
fn publish_writes_encoded_state() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    let (mut l, t) = polled(&fake);
    l.publish(&MemberState { in_midnight: true, has_relics: true, ready: false, level: 7, curse: 5 }, t);
    assert_eq!(fake.own_data(ROOM).as_deref(), Some("1,1,0,7,5"));
}

#[test]
fn unchanged_state_waits_for_resend() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    let (mut l, t) = polled(&fake);
    let s = MemberState::default();
    l.publish(&s, t);
    l.publish(&s, t + Duration::from_secs(4));
    assert_eq!(fake.sets.get(), 1);
    l.publish(&s, t + Duration::from_secs(5));
    assert_eq!(fake.sets.get(), 2);
}

#[test]
fn changed_state_publishes_at_once() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    let (mut l, t) = polled(&fake);
    l.publish(&MemberState::default(), t);
    l.publish(&MemberState { ready: true, ..Default::default() }, t + Duration::from_millis(100));
    assert_eq!(fake.sets.get(), 2);
}

/// 出击时换到出征大厅，房主的数据要晚一点才到，这段时间午夜和诅咒不能闪断
#[test]
fn new_lobby_keeps_host_state_until_data_arrives() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    fake.write(ROOM, HOST, "1,0,1,3,9");
    let (mut l, t) = polled(&fake);
    fake.enter(OTHER_ROOM, HOST, &[HOST, ME]);
    l.poll(t + POLL_EVERY);
    assert_eq!(l.host_state().map(|s| s.curse), Some(9));
    // 下一轮还没到也要继续沿用
    l.poll(t + POLL_EVERY * 2);
    assert_eq!(l.host_state().map(|s| s.curse), Some(9));
    fake.write(OTHER_ROOM, HOST, "0,0,0,3,4");
    l.poll(t + POLL_EVERY * 3);
    assert_eq!(l.host_state().map(|s| (s.in_midnight, s.curse)), Some((false, 4)));
}

#[test]
fn different_owner_does_not_inherit_host_state() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    fake.write(ROOM, HOST, "1,0,1,3,9");
    let (mut l, t) = polled(&fake);
    fake.enter(OTHER_ROOM, MATE, &[MATE, ME]);
    l.poll(t + POLL_EVERY);
    assert_eq!(l.host_state(), None);
}

#[test]
fn same_lobby_missing_data_is_not_inherited() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    fake.write(ROOM, HOST, "1,0,1,3,9");
    let (mut l, t) = polled(&fake);
    fake.data.borrow_mut().clear();
    l.poll(t + POLL_EVERY);
    assert_eq!(l.host_state(), None);
}

#[test]
fn new_lobby_publishes_at_once() {
    let fake = Fake::new();
    fake.enter(ROOM, HOST, &[HOST, ME]);
    let (mut l, t) = polled(&fake);
    let s = MemberState::default();
    l.publish(&s, t);
    fake.enter(OTHER_ROOM, ME, &[ME]);
    l.poll(t + POLL_EVERY);
    l.publish(&s, t + POLL_EVERY);
    assert!(fake.own_data(OTHER_ROOM).is_some());
}
