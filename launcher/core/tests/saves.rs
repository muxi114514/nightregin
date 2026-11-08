//! 确定存档账号：当前登录 > 记住的（目录还在）> 唯一的账号目录；只认 17 位数字目录

use std::fs;
use std::path::Path;

use launcher_core::saves::{SaveLocateError, SaveLocation, account_dirs, resolve};
use tempfile::TempDir;

const A: u64 = 76561198000000001;
/// 另一个账号
const B: u64 = 76561198000000009;
const SAVE: &str = "NR0000.co2";

fn make_account(root: &Path, id: u64) {
    fs::create_dir_all(root.join(id.to_string())).unwrap();
}

/// <root>\<id>\NR0000.co2
fn location(root: &Path, id: u64) -> SaveLocation {
    let dir = root.join(id.to_string());
    SaveLocation {
        account: id,
        file: dir.join(SAVE),
        dir,
    }
}

#[test]
fn resolve_prefers_active_account() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_account(root, A);
    make_account(root, B);
    assert_eq!(resolve(root, SAVE, Some(B), Some(A)), Ok(location(root, B)));
    assert_eq!(resolve(root, SAVE, Some(A), Some(B)), Ok(location(root, A)));
    assert_eq!(resolve(root, SAVE, Some(A), None), Ok(location(root, A)));
}

#[test]
fn resolve_uses_remembered_then_single_dir() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_account(root, A);
    make_account(root, B);
    // 没登录：用记住的
    assert_eq!(resolve(root, SAVE, None, Some(A)), Ok(location(root, A)));
    // 记住的账号目录没了，只剩一个账号目录：用剩下的
    fs::remove_dir(root.join(A.to_string())).unwrap();
    assert_eq!(resolve(root, SAVE, None, Some(A)), Ok(location(root, B)));
    assert_eq!(resolve(root, SAVE, None, None), Ok(location(root, B)));
}

#[test]
fn resolve_ambiguous_lists_accounts() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_account(root, B);
    make_account(root, A);
    let ambiguous = Err(SaveLocateError::Ambiguous(vec![A, B]));
    assert_eq!(resolve(root, SAVE, None, None), ambiguous);
    // 记住的账号没有目录：同样不猜
    assert_eq!(
        resolve(root, SAVE, None, Some(76561197960265729)),
        ambiguous
    );
}

#[test]
fn resolve_ignores_non_account_entries() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    // 本机根目录下就散落着这两个存档文件
    fs::write(root.join("NR0000.co2"), b"save").unwrap();
    fs::write(root.join("NR0000.sl2"), b"save").unwrap();
    // 17 位数字的文件
    fs::write(root.join("76561198000000002"), b"").unwrap();
    // 16 位、18 位、全角数字、带正号（u64 解析认 +）、夹字母
    let not_accounts = [
        "7656119800000000",
        "765611980000000001",
        "７６５６１１９８００００００００３",
        "+7656119800000004",
        "76561198O00000005",
    ];
    for name in not_accounts {
        fs::create_dir_all(root.join(name)).unwrap();
    }
    assert_eq!(account_dirs(root), Vec::<u64>::new());
    assert_eq!(
        resolve(root, SAVE, None, None),
        Err(SaveLocateError::NoAccounts)
    );

    make_account(root, B);
    assert_eq!(account_dirs(root), vec![B]);
    assert_eq!(resolve(root, SAVE, None, None), Ok(location(root, B)));
}

#[test]
fn active_account_without_dir_still_resolves() {
    let tmp = TempDir::new().unwrap();
    // 新玩家还没进过游戏：连 Nightreign 目录都没有
    let root = tmp.path().join("Nightreign");
    let want = location(&root, B);
    assert_eq!(want.file, root.join(r"76561198000000009\NR0000.co2"));
    assert_eq!(resolve(&root, SAVE, Some(B), None), Ok(want));
    // 没登录、记住的也没有目录：无从确定
    assert_eq!(
        resolve(&root, SAVE, None, Some(B)),
        Err(SaveLocateError::NoAccounts)
    );
    assert_eq!(
        resolve(&root, SAVE, None, None),
        Err(SaveLocateError::NoAccounts)
    );
    assert_eq!(account_dirs(&root), Vec::<u64>::new());
}
