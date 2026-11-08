//! 锻造石【３】金色强化（配置 [legendary_upgrade]）。
//! 原版锻造台把档位 2（紫色）的武器当作满级，【３】无处可用；而 +3 强化行与【３】的材料行原版 regulation 本来就有（与 MMV 逐行一致），
//! 限制只在两处代码：can_reinforce（锻造台列表 / 确认）与 make_next_record（预览、提交共用，生成 +1 后的物品记录），
//! 都是 `call get_stage; cmp al, 2; jae 不可强化`。把 jae 改成 ja，档位 2 走游戏自己的 +1 流程，档位 3（金色）与 9 仍不可强化。
//! 两个函数只被锻造台菜单调用（静态比对：4 处 / 2 处），不影响别处。
//! 与 MMV 的 RestoredLegendaryStones 共存：它改的是同一处 can_reinforce（已改则跳过）；它的预览 / 提交钩子只在
//! 目标 ID 仍等于原 ID 时才 +1，这里生成记录时已经 +1，它直接放行，不会重复强化。

use log::{info, warn};

use crate::config::LegendaryUpgradeConfig;
use crate::game::code_patch::write_code_byte;
use crate::game::memory::read;
use crate::game::module::GameModule;

const GET_STAGE_RVA: usize = 0x8BB780;
const CALL: u8 = 0xE8;
const CMP_AL_2: [u8; 2] = [0x3C, 0x02];
const JAE: u8 = 0x73;
const JA: u8 = 0x77;
/// jae 操作码相对 call 的偏移
const JUMP_OFFSET: usize = 7;

/// `call get_stage; cmp al, 2; jae rel8` 所在位置
struct Site {
    name: &'static str,
    call_rva: usize,
    jump_rel8: u8,
}

/// 先改生成记录、再改列表判断：中途失败时列表仍拦着档位 2，不会出现能选中却升不上去的状态
const SITES: [Site; 2] = [
    Site { name: "make_next_record", call_rva: 0x8BCE5E, jump_rel8: 0x46 },
    Site { name: "can_reinforce", call_rva: 0x8B8CAB, jump_rel8: 0x08 },
];

#[derive(Debug, PartialEq, Eq)]
enum JumpState {
    Original,
    /// 已被别的模组改成 ja（RestoredLegendaryStones）
    Patched,
    Mismatch,
}

pub fn apply(cfg: &LegendaryUpgradeConfig, module: &GameModule) {
    if !cfg.enabled {
        info!("legendary upgrade disabled by config");
        return;
    }
    let get_stage = module.va(GET_STAGE_RVA);
    let mut pending = Vec::with_capacity(SITES.len());
    for site in &SITES {
        let call = module.va(site.call_rva);
        let state = read::<[u8; 9]>(call).map_or(JumpState::Mismatch, |b| jump_state(b, call, get_stage, site.jump_rel8));
        match state {
            JumpState::Original => pending.push((site.name, call + JUMP_OFFSET)),
            JumpState::Patched => info!("legendary upgrade: {} already patched by another mod", site.name),
            JumpState::Mismatch => {
                warn!("legendary upgrade disabled: {} mismatch at rva {:#x}", site.name, site.call_rva);
                return;
            }
        }
    }
    for (i, &(name, at)) in pending.iter().enumerate() {
        if !write_code_byte(at, JA) {
            warn!("legendary upgrade disabled: patching {name} failed");
            // 撤回已改的，保持原版行为
            for &(_, done) in &pending[..i] {
                write_code_byte(done, JAE);
            }
            return;
        }
    }
    info!("legendary upgrade enabled: smithing stone [3] upgrades rare weapons ({} sites patched)", pending.len());
}

/// bytes：从 call 开始的 9 字节；call_va：call 指令地址
fn jump_state(bytes: [u8; 9], call_va: usize, get_stage_va: usize, jump_rel8: u8) -> JumpState {
    let rel = i32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
    let target = call_va.wrapping_add(5).wrapping_add_signed(rel as isize);
    if bytes[0] != CALL || target != get_stage_va || bytes[5..7] != CMP_AL_2 || bytes[8] != jump_rel8 {
        return JumpState::Mismatch;
    }
    match bytes[JUMP_OFFSET] {
        JAE => JumpState::Original,
        JA => JumpState::Patched,
        _ => JumpState::Mismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: usize = 0x1_4000_0000;
    // nightreign.exe 原始字节（静态读取，顺序同 SITES）
    const ORIGINAL: [[u8; 9]; 2] =
        [[0xE8, 0x1D, 0xE9, 0xFF, 0xFF, 0x3C, 0x02, 0x73, 0x46], [0xE8, 0xD0, 0x2A, 0x00, 0x00, 0x3C, 0x02, 0x73, 0x08]];

    fn state(bytes: [u8; 9], site: &Site) -> JumpState {
        jump_state(bytes, BASE + site.call_rva, BASE + GET_STAGE_RVA, site.jump_rel8)
    }

    #[test]
    fn recognizes_original_and_patched_jumps() {
        for (site, bytes) in SITES.iter().zip(ORIGINAL) {
            assert_eq!(state(bytes, site), JumpState::Original, "{}", site.name);
            let mut patched = bytes;
            patched[JUMP_OFFSET] = JA;
            assert_eq!(state(patched, site), JumpState::Patched, "{}", site.name);
        }
    }

    #[test]
    fn rejects_other_code() {
        let (site, bytes) = (&SITES[1], ORIGINAL[1]);
        // 调用目标不是 get_stage
        assert_eq!(jump_state(bytes, BASE + site.call_rva + 1, BASE + GET_STAGE_RVA, site.jump_rel8), JumpState::Mismatch);
        // 比较值、跳转距离、操作码不同
        let mut cmp3 = bytes;
        cmp3[6] = 3;
        assert_eq!(state(cmp3, site), JumpState::Mismatch);
        assert_eq!(jump_state(bytes, BASE + site.call_rva, BASE + GET_STAGE_RVA, 0x09), JumpState::Mismatch);
        let mut jb = bytes;
        jb[JUMP_OFFSET] = 0x72;
        assert_eq!(state(jb, site), JumpState::Mismatch);
    }
}
