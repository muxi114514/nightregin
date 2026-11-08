//! 黑夜君临逐级属性分配模组（me3 native DLL）。
//! 按 schemes.toml 计算本地角色每级的目标属性，用 regulation 中共享的组件 SpEffect（租约 + 续期）补齐差值。

mod app;
mod bootstrap;
mod boss_loot;
mod bridge;
mod config;
mod crucible_sword;
mod data;
mod diag;
mod editor;
pub mod game;
mod havok_guard;
mod hook;
mod hotkey;
pub mod lang;
mod legendary_upgrade;
mod logger;
mod midnight;
mod passive_inherit;
mod passive_upgrade;
mod relic_reroll;
mod rng;
mod rune_attack;
mod scheme_file;
mod sound_banks;
mod starting_weapons;
pub mod stats;
mod super_orb;
mod tiered_effects;
mod ui;

use std::ffi::c_void;

const DLL_PROCESS_ATTACH: u32 = 1;

/// 加载器回调：只启动初始化线程，DllMain 内不做任何重活（持有加载器锁）。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let module = module as usize;
        std::thread::spawn(move || bootstrap::run(module));
    }
    1
}
