//! 集成测试共用：fixture 路径、在临时目录搭假整合包
// 各测试文件只用到其中一部分，没用到的不算死代码
#![allow(dead_code)]

pub mod backup;
pub mod fake_os;
pub mod minidump;

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use launcher_core::paths::PROFILE_FILE;
use launcher_core::profile::Me3Profile;
use tempfile::TempDir;

/// 指向 tests\fixtures\<rel>
pub fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

/// 临时目录里的假游戏 + 整合包；tmp 析构时整个删除
pub struct FakePack {
    pub tmp: TempDir,
    pub game_root: PathBuf,
    pub pack_root: PathBuf,
}

/// 建 <tmp>\<dir_name>\Game\nightreign.exe 和 <tmp>\<dir_name>\me3\MMV\me3\：
/// 复制 fixtures\pack 全部内容，再按 .me3 补齐 bin 下三个文件、natives（空文件）和 packages（空目录）
pub fn fake_pack(dir_name: &str) -> FakePack {
    let tmp = TempDir::new().unwrap();
    let game_root = tmp.path().join(dir_name);
    let pack_root = game_root.join("me3").join("MMV").join("me3");
    touch(&game_root.join("Game").join("nightreign.exe"));
    copy_dir(&fixture("pack"), &pack_root);
    for name in ["me3.exe", "me3-launcher.exe", "me3_mod_host.dll"] {
        touch(&pack_root.join("bin").join(name));
    }
    let profile = Me3Profile::load(&pack_root.join(PROFILE_FILE)).unwrap();
    for native in &profile.natives {
        touch(&native.path);
    }
    for package in &profile.packages {
        fs::create_dir_all(&package.path).unwrap();
    }
    FakePack {
        tmp,
        game_root,
        pack_root,
    }
}

/// 建空文件（连同父目录）；已存在的不截断
fn touch(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}
