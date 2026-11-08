//! 帧时间监测：每 10 秒汇总一次卡顿帧（明显长于平均的帧），统计其中紧跟在本模组原生调用
//! （增删 / 续期 SpEffect）之后的数量，以及本模组每帧自身耗时，用来判断卡顿是否由本模组引起。

use std::time::{Duration, Instant};

use log::info;

const REPORT_EVERY: Duration = Duration::from_secs(10);
/// 超过这个间隔视为加载画面，不计入统计
const LOADING_GAP: Duration = Duration::from_secs(1);
/// 卡顿帧：超过平均的 2 倍且至少 33 ms
const SPIKE_MIN: Duration = Duration::from_millis(33);
/// 原生调用后这段时间内出现的卡顿算作"紧跟其后"
const NEAR_NATIVE: Duration = Duration::from_millis(150);
/// 平均值至少基于这么多帧才开始判定卡顿
const WARMUP_FRAMES: u32 = 30;

#[derive(Default)]
pub struct FrameMonitor {
    last_frame: Option<Instant>,
    last_native: Option<Instant>,
    window_start: Option<Instant>,
    frames: u32,
    total: Duration,
    spikes: u32,
    spikes_after_native: u32,
    worst: Duration,
    natives: u32,
    own_total: Duration,
    own_max: Duration,
}

impl FrameMonitor {
    /// 每帧开头调用
    pub fn frame_start(&mut self, now: Instant) {
        if let Some(prev) = self.last_frame.replace(now) {
            let dt = now - prev;
            if dt < LOADING_GAP {
                self.record_frame(now, dt);
            }
        }
        let start = *self.window_start.get_or_insert(now);
        if now - start >= REPORT_EVERY {
            self.report();
            *self = Self { last_frame: self.last_frame, last_native: self.last_native, window_start: Some(now), ..Default::default() };
        }
    }

    /// 本模组本帧的耗时（每帧末尾调用）
    pub fn frame_cost(&mut self, cost: Duration) {
        self.own_total += cost;
        self.own_max = self.own_max.max(cost);
    }

    /// 每批调用游戏的 SpEffect 增删函数时调用一次
    pub fn native_call(&mut self) {
        self.last_native = Some(Instant::now());
        self.natives += 1;
    }

    fn record_frame(&mut self, now: Instant, dt: Duration) {
        if self.frames >= WARMUP_FRAMES {
            let avg = self.total / self.frames;
            if dt > avg * 2 && dt > SPIKE_MIN {
                self.spikes += 1;
                self.worst = self.worst.max(dt);
                if self.last_native.is_some_and(|t| now.saturating_duration_since(t) <= dt + NEAR_NATIVE) {
                    self.spikes_after_native += 1;
                }
            }
        }
        self.frames += 1;
        self.total += dt;
    }

    /// 有卡顿或本模组单帧耗时超过 2 ms 时才写日志
    fn report(&self) {
        if self.frames == 0 || (self.spikes == 0 && self.own_max < Duration::from_millis(2)) {
            return;
        }
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        info!(
            "perf 10s: {} frames avg {:.1} ms; {} spikes (worst {:.0} ms), {} right after our {} SpEffect batches; mod cost avg {:.0} us, max {:.0} us",
            self.frames,
            ms(self.total / self.frames),
            self.spikes,
            ms(self.worst),
            self.spikes_after_native,
            self.natives,
            ms(self.own_total / self.frames) * 1000.0,
            ms(self.own_max) * 1000.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spike_after_native_call_is_attributed() {
        let mut m = FrameMonitor::default();
        let t0 = Instant::now();
        let frame = Duration::from_millis(16);
        for i in 0..40 {
            m.record_frame(t0 + frame * i, frame);
        }
        m.native_call();
        m.record_frame(Instant::now() + Duration::from_millis(80), Duration::from_millis(80));
        assert_eq!((m.spikes, m.spikes_after_native), (1, 1));
        m.last_native = None;
        m.record_frame(Instant::now(), Duration::from_millis(90));
        assert_eq!((m.spikes, m.spikes_after_native), (2, 1));
    }
}
