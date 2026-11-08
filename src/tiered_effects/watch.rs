//! 诊断（cargo 特性 tiered-diag）：出击中每帧比对主角身上指定效果的出现 / 消失写进日志，并每 30 秒记一次全部效果，
//! 用来排查每帧动作脚本链走到哪一步（打卡效果 9971788~797）、动作文件里的事件有没有触发（探针 9971780~783），
//! 以及锻体标记、原版状态是否在身上。打卡与探针由诊断脚本注入本机测试文件。
//! 第六感：在 PlayerCommonParam 的「可用 / 已用」效果号字段上下硬件断点（game::hw_breakpoint），记下读到它们的代码；
//! 这两个效果出现或消失时，比对前后约 0.5 秒的主角内存（mem_diff），找引擎记录第六感次数的位置。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use log::info;
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::player_common;

use crate::game::hw_breakpoint;
use crate::game::memory::read_bytes;
use crate::game::params::ParamRepository;

use super::mem_diff::MemDiff;

/// (效果, 说明)
const WATCHED: &[(i32, &str)] = &[
    (9_971_788, "HKS checkpoint: Update start"),
    (9_971_789, "HKS checkpoint: Update before TieredUpdate"),
    (9_971_790, "HKS checkpoint: TieredUpdate start"),
    (9_971_791, "HKS checkpoint: after TieredKeepApplied"),
    (9_971_792, "HKS checkpoint: after TieredUpdateAura"),
    (9_971_793, "HKS checkpoint: Roles after Resonance"),
    (9_971_794, "HKS checkpoint: Roles after Residues"),
    (9_971_795, "HKS checkpoint: Roles after Actions"),
    (9_971_796, "HKS checkpoint: TieredUpdate end"),
    (9_971_797, "HKS checkpoint: ExecEvent wrapper"),
    (9_971_780, "TAE probe: T67 unconditional in cursed sword anims"),
    (9_971_781, "TAE probe: self bullet, state 609 (drawn)"),
    (9_971_782, "TAE probe: self bullet, state 2393 (Executor7 marker)"),
    (9_971_783, "TAE probe: self bullet, unconditional"),
    (9_971_140, "marker: purple6 empty flask negation"),
    (9_971_141, "applied: purple6 negation"),
    (9_971_350, "marker: Executor4 beast HP lock"),
    (9_971_351, "applied: Executor4 undying"),
    (9_971_650, "marker: Executor7 cursed sword ailments (state 2393)"),
    (9_971_760, "marker: Executor2 longer deflect"),
    (9_971_761, "applied: Executor2 deflect extension"),
    (9_971_765, "marker: Executor3 one deflect release"),
    (9_971_766, "applied: Executor3 accumulation +50"),
    (9_971_770, "marker: Executor6 roar heal"),
    (9_971_771, "applied: Executor6 extra heal"),
    (9_971_740, "marker: Guardian9 roll"),
    (9_971_750, "marker: Raider1 enhanced skill"),
    (9_971_755, "marker: Raider5 heavy armor"),
    (9_971_756, "applied: Raider5 armor"),
    (9_971_775, "marker: Scholar6 long barrier"),
    (9_971_776, "applied: Scholar6 barrier extension"),
    (704_011, "vanilla: Raider skill enhanced ready"),
    (707_000, "vanilla: Executor just guard window"),
    (707_051, "vanilla: Executor cursed sword release ready"),
    (707_060, "vanilla: Executor cursed sword drawn (state 609)"),
    (707_100, "vanilla: Executor beast form (named Is Beast Form)"),
    (707_115, "vanilla: Executor beast health boost (IsDemonBeastMode)"),
    (9_971_352, "applied: Executor4 heal"),
    (708_050, "vanilla: Scholar parry barrier"),
    (7_011_701, "vanilla: Executor relic roar heal"),
    (700_600, "vanilla: Wylder Sixth Sense ready"),
    (700_601, "vanilla: Wylder Sixth Sense used"),
    (9_971_850, "marker: Wylder12 extra Sixth Sense"),
    (106, "vanilla: grace reload"),
    (102_190, "vanilla: revived"),
    (9_971_260, "marker: Gold6 empowered Cold Mirage"),
    (8_970_040, "vanilla: Cold Mirage ready"),
    (8_970_041, "vanilla: Cold Mirage active (invulnerable, concealed)"),
    (8_970_042, "vanilla: Cold Mirage used"),
    (9_971_262, "applied: Gold6 buff"),
    (9_971_263, "Gold6 ready"),
    (9_971_264, "Gold6 used"),
    (9_971_070, "marker: Blue10 quick forge"),
    // 执行者8 熔炉形态绝招（tools\tiered_crucible.lua）：入口扣量 → 拔刀 → 踏地状态（原版状态每帧标记）→ 动作事件挂形态
    (9_971_865, "marker: Executor8 crucible sword"),
    (9_971_871, "crucible: consume gauge (ult entry)"),
    (9_971_873, "crucible: poise"),
    (9_971_870, "crucible: form (from stomp TAE at 1.03s, or script fallback)"),
    (9_971_872, "crucible: drain"),
    (9_971_879, "crucible: active (DLL forces moveset 293)"),
    (9_971_880, "crucible: golden wrath spawn (stomp TAE / end)"),
    (6_999_615, "vanilla: ultimate active"),
    (103_238, "vanilla: state AttackBothLight6 (stomp slot 32050)"),
    (103_206, "vanilla: state AttackRightLightStep (old slot 30300)"),
];
/// 内存参数表里第六感两个效果号字段的地址（硬件断点用）；启动时定位
pub fn sixth_sense_fields(repository: &ParamRepository) -> Vec<usize> {
    let table = player_common::TABLE;
    let Some((address, size)) = repository.table_locations(table.struct_name).into_iter().next() else { return Vec::new() };
    let Some(bytes) = read_bytes(address, size as usize) else { return Vec::new() };
    let Ok(file) = ParamFile::parse(&bytes, table.row_size) else { return Vec::new() };
    file.rows()
        .find(|r| r.id == player_common::ROW)
        .map(|r| vec![address + r.offset + player_common::SIXTH_SENSE_EFFECT, address + r.offset + player_common::SIXTH_SENSE_USED_EFFECT])
        .unwrap_or_default()
}

/// 出现或消失时做内存比对的效果
const DIFF_TRIGGERS: &[i32] = &[700_600, 700_601];
const DUMP_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct Watch {
    present: Vec<bool>,
    next_dump: Option<Instant>,
    /// 要下硬件断点的地址；每次出击装一次（补上新建的线程）
    breakpoints: Vec<usize>,
    installed: bool,
    /// 已写进日志的断点命中：RVA → 次数
    reported: HashMap<usize, u32>,
    diff: MemDiff,
}

impl Watch {
    pub fn new(breakpoints: Vec<usize>) -> Self {
        Self { breakpoints, ..Self::default() }
    }

    pub fn observe(&mut self, ids: &[i32], player: usize, game_data: usize) {
        if self.present.len() != WATCHED.len() {
            self.present = vec![false; WATCHED.len()];
            info!("diag: watching {} effects", WATCHED.len());
        }
        self.install_breakpoints();
        self.diff.capture(player, game_data);
        let mut changed = false;
        for (seen, (id, what)) in self.present.iter_mut().zip(WATCHED) {
            let now = ids.contains(id);
            if now != *seen {
                let sign = if now { "+" } else { "-" };
                info!("diag: {sign} {id} ({what})");
                if DIFF_TRIGGERS.contains(id) {
                    self.diff.mark(format!("{sign}{id}"));
                    changed = true;
                }
                *seen = now;
            }
        }
        self.log_breakpoints(changed);
        let now = Instant::now();
        if self.next_dump.is_none_or(|t| now >= t) {
            let mut sorted = ids.to_vec();
            sorted.sort_unstable();
            info!("diag: {} effects on player: {sorted:?}", sorted.len());
            self.log_breakpoints(true);
            self.next_dump = Some(now + DUMP_INTERVAL);
        }
    }

    pub fn reset(&mut self) {
        self.present.clear();
        self.next_dump = None;
        self.installed = false;
        self.diff.reset();
    }

    fn install_breakpoints(&mut self) {
        if self.installed || self.breakpoints.is_empty() {
            return;
        }
        self.installed = true;
        let addrs = self.breakpoints.clone();
        // 不能改自己正在跑的线程，另起一个线程去装
        std::thread::spawn(move || {
            let n = hw_breakpoint::install(&addrs);
            info!("diag: hardware breakpoints on {addrs:x?} installed in {n} threads");
        });
    }

    /// 新出现的命中地址立刻记；all 时把次数有变化的都记一遍
    fn log_breakpoints(&mut self, all: bool) {
        for (rva, hits, which) in hw_breakpoint::hits() {
            let old = self.reported.get(&rva).copied();
            if old.is_none() || (all && old != Some(hits)) {
                info!("diag: hwbp rva 0x{rva:x} (bp {which:#b}) hits {hits}{}", if old.is_none() { " (new)" } else { "" });
                self.reported.insert(rva, hits);
            }
        }
    }
}
