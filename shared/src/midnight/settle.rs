//! 一局午夜的结算状态机：圆桌→出征记「出击」，出征中记录击败旗标，出征→圆桌给出胜负。
//! 位置读不到（读图过渡）时不改变状态，所以过渡期间不会误结算。

use super::Outcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Hub,
    Expedition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettleEvent {
    None,
    /// 本局计午夜，进度文件要记「进行中」
    Sortied,
    Settled(Outcome),
}

/// 夜王击败旗标（NightBossMenuParam.defeatEventFlag）：普通 150~162，永夜 170~181
pub fn boss_flag_kind(flag: u32) -> Option<bool> {
    match flag {
        150..=162 => Some(false),
        170..=181 => Some(true),
        _ => None,
    }
}

/// 启动时进度文件还留着「进行中」= 上次闪退或掉线，这局作废
pub fn resume(in_progress: bool) -> Option<Outcome> {
    in_progress.then_some(Outcome::Void)
}

#[derive(Debug, Default)]
pub struct Settler {
    last: Option<Place>,
    /// 本局正在计分
    running: bool,
    /// 本局看到的击败旗标：None 未见，Some(是否永夜)
    defeated: Option<bool>,
}

impl Settler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_place(&mut self, place: Option<Place>, counted: bool) -> SettleEvent {
        let Some(place) = place else { return SettleEvent::None };
        let prev = self.last.replace(place);
        match (prev, place) {
            (Some(Place::Hub), Place::Expedition) if counted => {
                self.running = true;
                self.defeated = None;
                SettleEvent::Sortied
            }
            (Some(Place::Expedition), Place::Hub) if self.running => {
                self.running = false;
                let outcome = match self.defeated.take() {
                    Some(everdark) => Outcome::Win { everdark },
                    None => Outcome::Loss,
                };
                SettleEvent::Settled(outcome)
            }
            _ => SettleEvent::None,
        }
    }

    pub fn on_boss_flag(&mut self, flag: u32) {
        let Some(everdark) = boss_flag_kind(flag) else { return };
        if self.running {
            self.defeated = Some(self.defeated.unwrap_or(false) || everdark);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midnight::Outcome;

    fn run(s: &mut Settler, steps: &[Option<Place>], counted: bool) -> Vec<SettleEvent> {
        steps.iter().map(|&p| s.on_place(p, counted)).collect()
    }

    #[test]
    fn win_after_boss_flag() {
        let mut s = Settler::new();
        run(&mut s, &[Some(Place::Hub), Some(Place::Expedition)], true);
        s.on_boss_flag(151);
        assert_eq!(s.on_place(Some(Place::Hub), true), SettleEvent::Settled(Outcome::Win { everdark: false }));
    }

    #[test]
    fn everdark_win() {
        let mut s = Settler::new();
        run(&mut s, &[Some(Place::Hub), Some(Place::Expedition)], true);
        s.on_boss_flag(175);
        assert_eq!(s.on_place(Some(Place::Hub), true), SettleEvent::Settled(Outcome::Win { everdark: true }));
    }

    #[test]
    fn everdark_wins_over_normal() {
        let mut s = Settler::new();
        run(&mut s, &[Some(Place::Hub), Some(Place::Expedition)], true);
        s.on_boss_flag(151);
        s.on_boss_flag(175);
        s.on_boss_flag(152);
        assert_eq!(s.on_place(Some(Place::Hub), true), SettleEvent::Settled(Outcome::Win { everdark: true }));
    }

    #[test]
    fn sortie_is_reported() {
        let mut s = Settler::new();
        assert_eq!(run(&mut s, &[Some(Place::Hub), Some(Place::Expedition)], true), vec![SettleEvent::None, SettleEvent::Sortied]);
    }

    #[test]
    fn loss_without_flag() {
        let mut s = Settler::new();
        let ev = run(&mut s, &[Some(Place::Hub), Some(Place::Expedition), Some(Place::Hub)], true);
        assert_eq!(ev.last(), Some(&SettleEvent::Settled(Outcome::Loss)));
    }

    #[test]
    fn boss_flag_outside_expedition_ignored() {
        let mut s = Settler::new();
        s.on_place(Some(Place::Hub), true);
        s.on_boss_flag(151);
        let ev = run(&mut s, &[Some(Place::Expedition), Some(Place::Hub)], true);
        assert_eq!(ev.last(), Some(&SettleEvent::Settled(Outcome::Loss)));
    }

    #[test]
    fn none_place_never_settles() {
        let mut s = Settler::new();
        run(&mut s, &[Some(Place::Hub), Some(Place::Expedition)], true);
        assert_eq!(run(&mut s, &[None, None], true), vec![SettleEvent::None, SettleEvent::None]);
        assert_eq!(s.on_place(Some(Place::Hub), true), SettleEvent::Settled(Outcome::Loss));
    }

    #[test]
    fn uncounted_sortie_is_ignored() {
        let mut s = Settler::new();
        let ev = run(&mut s, &[Some(Place::Hub), Some(Place::Expedition), Some(Place::Hub)], false);
        assert!(ev.iter().all(|e| *e == SettleEvent::None));
    }

    #[test]
    fn first_place_expedition_does_not_sortie() {
        // DLL 在出征中途才开始跟踪（例如刚装上），不算这局
        let mut s = Settler::new();
        let ev = run(&mut s, &[Some(Place::Expedition), Some(Place::Hub)], true);
        assert!(ev.iter().all(|e| *e == SettleEvent::None));
    }

    #[test]
    fn crash_resume_is_void() {
        assert_eq!(resume(true), Some(Outcome::Void));
        assert_eq!(resume(false), None);
    }

    #[test]
    fn boss_flag_ranges() {
        assert_eq!(boss_flag_kind(150), Some(false));
        assert_eq!(boss_flag_kind(162), Some(false));
        assert_eq!(boss_flag_kind(163), None);
        assert_eq!(boss_flag_kind(170), Some(true));
        assert_eq!(boss_flag_kind(181), Some(true));
        assert_eq!(boss_flag_kind(182), None);
    }
}
