//! 卢恩类锻体的判断：只看本帧状态与自己的记录，返回改动后的卢恩；施加效果走回调，便于单测。

use std::time::{Duration, Instant};

use log::info;
use nr_stat_alloc_shared::tiered_runes::{
    RUNE_GAMBLE, RUNE_REFUND, RUNE_SHIELD, RUNE_SHIELD_BLAST, RUNE_SHIELD_HEAL, RUNE_SHIELD_UNDYING, gamble_multiplier, gambled,
};

use crate::config::TieredBuffsConfig;
use crate::rng::XorShift;

use super::Frame;

/// 死亡 / 倒地后这段时间内卢恩减少不算花费（死亡掉落）
const AFTER_DOWN: Duration = Duration::from_secs(10);
/// 免死触发后的间隔，防止同一次伤害在下一帧还是 1 血时重复扣卢恩
const SHIELD_COOLDOWN: Duration = Duration::from_secs(1);

/// [0, 1) 的均匀随机数
fn unit(rng: &mut XorShift) -> f64 {
    (rng.next() >> 11) as f64 / (1u64 << 53) as f64
}

/// 蓝1：每拿一次身上多一份同号效果；份数比本局见过的最多还多时，每多一份抽一次倍数（读档 / 换图时份数暂时变少不算）
#[derive(Default)]
pub struct Gamble {
    seen: Option<usize>,
}

impl Gamble {
    pub fn check(&mut self, frame: &Frame, runes: u32, cfg: &TieredBuffsConfig, rng: &mut XorShift) -> Option<u32> {
        let count = frame.ids.iter().filter(|&&id| id == RUNE_GAMBLE).count();
        // 第一次看到（刚出击或 DLL 中途加载）只记下份数
        let Some(seen) = self.seen else {
            self.seen = Some(count);
            return None;
        };
        if count <= seen {
            return None;
        }
        self.seen = Some(count);
        let mut held = runes;
        for _ in seen..count {
            let m = gamble_multiplier(unit(rng), cfg.rune_gamble_min, cfg.rune_gamble_max);
            let next = gambled(held, m, cfg.rune_gamble_cap);
            info!("rune gamble: {held} x{m:.2} -> {next}");
            held = next;
        }
        Some(held)
    }

    pub fn reset(&mut self) {
        self.seen = None;
    }
}

/// 紫13：活着时卢恩比上一帧少（升级、购买、锻造）按概率退回；死亡掉落不算
#[derive(Default)]
pub struct Refund {
    last: Option<u32>,
    blocked_until: Option<Instant>,
}

impl Refund {
    pub fn check(&mut self, frame: &Frame, cfg: &TieredBuffsConfig, rng: &mut XorShift) -> Option<u32> {
        let now = Instant::now();
        if frame.hp.0 == 0 {
            self.blocked_until = Some(now + AFTER_DOWN);
        }
        let prev = self.last?;
        let alive = frame.hp.0 > 0 && self.blocked_until.is_none_or(|t| now >= t);
        if !frame.ids.contains(&RUNE_REFUND) || !alive || frame.runes >= prev {
            return None;
        }
        let spent = prev - frame.runes;
        if unit(rng) >= cfg.rune_refund_chance {
            info!("rune refund: spent {spent}, not refunded");
            return None;
        }
        info!("rune refund: spent {spent}, refunded");
        Some(prev)
    }

    /// 本帧结束时的卢恩（倍数、免死改动之后），下一帧和它比
    pub fn remember(&mut self, runes: u32) {
        self.last = Some(runes);
    }

    pub fn reset(&mut self) {
        self.last = None;
        self.blocked_until = None;
    }
}

/// 金8：卢恩够时保持短时不死；血量被压到 1 时扣卢恩并回满血
#[derive(Default)]
pub struct Shield {
    ready_at: Option<Instant>,
}

impl Shield {
    pub fn check(&mut self, frame: &Frame, runes: u32, cfg: &TieredBuffsConfig, mut apply: impl FnMut(i32)) -> Option<u32> {
        if !frame.ids.contains(&RUNE_SHIELD) || runes < cfg.rune_shield_cost {
            return None;
        }
        let (hp, max) = frame.hp;
        let undying = frame.ids.contains(&RUNE_SHIELD_UNDYING);
        let now = Instant::now();
        if undying && hp <= 1 && max > 1 && self.ready_at.is_none_or(|t| now >= t) {
            apply(RUNE_SHIELD_HEAL);
            apply(RUNE_SHIELD_BLAST);
            self.ready_at = Some(now + SHIELD_COOLDOWN);
            info!("rune shield: lethal damage averted, {} runes consumed", cfg.rune_shield_cost);
            return Some(runes - cfg.rune_shield_cost);
        }
        if !undying && hp > 0 {
            apply(RUNE_SHIELD_UNDYING);
        }
        None
    }

    pub fn reset(&mut self) {
        self.ready_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(chance: f64) -> TieredBuffsConfig {
        TieredBuffsConfig { rune_refund_chance: chance, ..TieredBuffsConfig::default() }
    }

    fn frame(ids: &[i32], hp: u32, runes: u32) -> Frame<'_> {
        Frame { ids, hp: (hp, 1000), runes }
    }

    #[test]
    fn gamble_triggers_only_on_new_copies() {
        let (mut g, mut rng, c) = (Gamble::default(), XorShift::from_time(), cfg(0.4));
        assert_eq!(g.check(&frame(&[], 500, 1000), 1000, &c, &mut rng), None);
        assert!(g.check(&frame(&[RUNE_GAMBLE], 500, 1000), 1000, &c, &mut rng).is_some());
        // 换图时效果暂时消失再回来，不重复触发
        assert_eq!(g.check(&frame(&[], 500, 1000), 1000, &c, &mut rng), None);
        assert_eq!(g.check(&frame(&[RUNE_GAMBLE], 500, 1000), 1000, &c, &mut rng), None);
        let r = g.check(&frame(&[RUNE_GAMBLE, RUNE_GAMBLE], 500, 1000), 1000, &c, &mut rng).unwrap();
        assert!((100..=10_000).contains(&r));
    }

    #[test]
    fn refund_only_spending_while_alive() {
        let (mut f, mut rng) = (Refund::default(), XorShift::from_time());
        let ids = [RUNE_REFUND];
        f.remember(5000);
        assert_eq!(f.check(&frame(&ids, 500, 3000), &cfg(1.0), &mut rng), Some(5000));
        f.remember(3000);
        assert_eq!(f.check(&frame(&ids, 500, 2000), &cfg(0.0), &mut rng), None);
        // 倒地后掉卢恩不退
        f.remember(2000);
        assert_eq!(f.check(&frame(&ids, 0, 0), &cfg(1.0), &mut rng), None);
        f.remember(0);
        assert_eq!(f.check(&frame(&[], 500, 0), &cfg(1.0), &mut rng), None);
    }

    #[test]
    fn shield_keeps_undying_and_pays_on_one_hp() {
        let (mut s, c) = (Shield::default(), cfg(0.4));
        let mut applied = Vec::new();
        assert_eq!(s.check(&frame(&[RUNE_SHIELD], 800, 30_000), 30_000, &c, |id| applied.push(id)), None);
        assert_eq!(applied, vec![RUNE_SHIELD_UNDYING]);
        applied.clear();
        let hit = frame(&[RUNE_SHIELD, RUNE_SHIELD_UNDYING], 1, 30_000);
        assert_eq!(s.check(&hit, 30_000, &c, |id| applied.push(id)), Some(10_000));
        assert_eq!(applied, vec![RUNE_SHIELD_HEAL, RUNE_SHIELD_BLAST]);
        // 冷却中不重复扣；卢恩不够时不再挂不死
        assert_eq!(s.check(&hit, 10_000, &c, |id| applied.push(id)), None);
        assert_eq!(applied.len(), 2);
    }
}
