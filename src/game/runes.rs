//! 局内持有的卢恩：PlayerGameData+0x6C 的 u32（同 ER；exe 里利普拉「持有卢恩越多攻击越高」在 rva 0x6C32A7 `mov ebx, [r13+0x6C]` 读的就是它）。

use super::memory::{read, write};

const HELD: usize = 0x6C;

pub fn held(game_data: usize) -> Option<u32> {
    read::<u32>(game_data + HELD)
}

pub fn set(game_data: usize, runes: u32) -> bool {
    write(game_data + HELD, runes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_held_runes() {
        let mut pgd = [0u32; 0x20];
        pgd[HELD / 4] = 12_345;
        let gd = pgd.as_mut_ptr() as usize;
        assert_eq!(held(gd), Some(12_345));
        assert!(set(gd, 99));
        assert_eq!(pgd[HELD / 4], 99);
    }
}
