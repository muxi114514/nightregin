//! SteamID64 换算；读注册表、找 Steam 库依赖本机环境，不在这里测

use launcher_core::steam::{STEAM_ID64_BASE, steam_id64};

#[test]
fn steam_id64_from_account() {
    // 示例账号：ActiveUser = 0x25E4C09，对应存档目录 76561198000000009
    assert_eq!(steam_id64(0x25E4C09), Some(76561198000000009));
    // Steam 开着但没登录
    assert_eq!(steam_id64(0), None);
    // 账号 ID 的整个取值范围都落在 17 位数字内（账号目录只认 17 位）
    assert_eq!(steam_id64(1), Some(STEAM_ID64_BASE + 1));
    assert_eq!(
        steam_id64(u32::MAX).map(|id| id.to_string().len()),
        Some(17)
    );
}
