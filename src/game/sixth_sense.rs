//! 追踪者第六感的剩余次数。PlayerIns+0x1B8（角色模块表）+0x1A0 是第六感组件，组件 +0x10 的字节是次数：
//! 引擎每帧（rva 0x4433C0）按它同步效果，> 0 时确保挂「可用」700600、为 0 时确保挂「已用」700601，触发时（0x4431F0）只看身上有没有「可用」，
//! 所以单独挂回「可用」会在下一帧被「已用」顶掉。PlayerGameData+0xDA 是次数的存档镜像，组件重建时（0x4432C0）读回。

use super::memory::{read, read_ptr, write};

const CHR_MODULES: usize = 0x1B8;
const SIXTH_SENSE_MODULE: usize = 0x1A0;
const COUNT: usize = 0x10;
const GAME_DATA_COUNT: usize = 0xDA;

fn module(player: usize) -> Option<usize> {
    read_ptr(player + CHR_MODULES).and_then(|m| read_ptr(m + SIXTH_SENSE_MODULE)).filter(|&c| c != 0)
}

pub fn count(player: usize) -> Option<u8> {
    module(player).and_then(|c| read::<u8>(c + COUNT))
}

/// 改组件里的次数，并同步存档镜像
pub fn set_count(player: usize, game_data: usize, n: u8) -> bool {
    module(player).is_some_and(|c| write(c + COUNT, n)) && write(game_data + GAME_DATA_COUNT, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_module_chain() {
        let mut component = [0u8; 0x20];
        let mut modules = [0usize; 0x40];
        let mut player = [0usize; 0x40];
        let mut game_data = [0u8; 0x100];
        component[COUNT] = 1;
        modules[SIXTH_SENSE_MODULE / 8] = component.as_mut_ptr() as usize;
        player[CHR_MODULES / 8] = modules.as_mut_ptr() as usize;
        let p = player.as_mut_ptr() as usize;
        assert_eq!(count(p), Some(1));
        component[COUNT] = 0;
        assert!(set_count(p, game_data.as_mut_ptr() as usize, 1));
        assert_eq!((component[COUNT], game_data[GAME_DATA_COUNT]), (1, 1));
    }
}
