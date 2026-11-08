//! 游戏内随机抽取用的 xorshift64（不需要密码学强度），以启动时间为种子。

use std::time::{SystemTime, UNIX_EPOCH};

pub struct XorShift(u64);

impl XorShift {
    pub fn from_time() -> Self {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        Self(seed | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}
