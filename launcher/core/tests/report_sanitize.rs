//! 问题记录脱敏：用户名的各种写法、SteamID64、短用户名不打烂日志

use launcher_core::report::sanitize::{Identities, Sanitizer};

const USER: &str = "<用户名>";
const ID: &str = "<SteamID>";

fn sanitizer(names: &[&[u8]], steam_ids: &[u64]) -> Sanitizer {
    Sanitizer::new(&Identities {
        names: names.iter().map(|n| n.to_vec()).collect(),
        steam_ids: steam_ids.to_vec(),
    })
}

fn apply(s: &Sanitizer, text: &str) -> String {
    String::from_utf8(s.apply(text.as_bytes())).unwrap()
}

#[test]
fn sanitize_username_case_insensitive() {
    let s = sanitizer(&[b"Administrator"], &[]);
    assert_eq!(
        apply(&s, r"C:\Users\Administrator\x ADMINISTRATOR administrator"),
        format!(r"C:\Users\{USER}\x {USER} {USER}")
    );
}

#[test]
fn sanitize_short_path_form() {
    let s = sanitizer(&[b"Administrator", b"ADMINI~1"], &[]);
    assert_eq!(
        apply(&s, r"C:\Users\ADMINI~1\AppData\Local\Temp"),
        format!(r"C:\Users\{USER}\AppData\Local\Temp")
    );
}

#[test]
fn sanitize_gbk_and_utf8_chinese_name() {
    let (utf8, gbk): (&[u8], &[u8]) = ("张三".as_bytes(), b"\xD5\xC5\xC8\xFD");
    let s = sanitizer(&[utf8, gbk], &[]);
    assert_eq!(apply(&s, r"C:\Users\张三\a"), format!(r"C:\Users\{USER}\a"));
    let mut bytes = b"C:\\Users\\".to_vec();
    bytes.extend_from_slice(gbk);
    bytes.extend_from_slice(b"\\a");
    let mut want = b"C:\\Users\\".to_vec();
    want.extend_from_slice(USER.as_bytes());
    want.extend_from_slice(b"\\a");
    assert_eq!(s.apply(&bytes), want);
}

#[test]
fn sanitize_utf16_name() {
    let name: Vec<u8> = "Bob1".encode_utf16().flat_map(u16::to_le_bytes).collect();
    let s = sanitizer(&[&name], &[]);
    let mut input = vec![0x41, 0x00];
    input.extend_from_slice(&name);
    let out = s.apply(&input);
    assert!(!out.windows(name.len()).any(|w| w == name.as_slice()));
}

#[test]
fn sanitize_steam_id_any_account() {
    let s = sanitizer(&[], &[]);
    assert_eq!(apply(&s, "id=76561198000000001;"), format!("id={ID};"));
    // 好友的也换
    assert_eq!(apply(&s, "76561199999999999"), ID);
    for keep in [
        "7656119800000000",
        "765611980000000012",
        "12345678901234567",
    ] {
        assert_eq!(apply(&s, keep), keep);
    }
}

#[test]
fn sanitize_account_id_low_32() {
    // 76561198000000001 - 76561197960265728 = 39734273
    let s = sanitizer(&[], &[76561198000000001]);
    assert_eq!(apply(&s, "[U:1:39734273] x"), format!("[U:1:{ID}] x"));
    assert_eq!(apply(&s, "397342730"), "397342730");
}

#[test]
fn sanitize_short_name_only_in_path_context() {
    let s = sanitizer(&[b"ab"], &[]);
    assert_eq!(apply(&s, "cabin about ab"), "cabin about ab");
    assert_eq!(apply(&s, r"C:\Users\ab\x"), format!(r"C:\Users\{USER}\x"));
    assert_eq!(apply(&s, r"C:/Users/AB/x"), format!(r"C:/Users/{USER}/x"));
    assert_eq!(
        apply(&s, r"C:\\Users\\ab\\x"),
        format!(r"C:\\Users\\{USER}\\x")
    );
    // 只是前缀，不是这个用户名
    assert_eq!(apply(&s, r"C:\Users\abc\x"), r"C:\Users\abc\x");
}

#[test]
fn sanitize_binary_safe() {
    let s = sanitizer(&[b"Administrator"], &[]);
    let input = b"\xFF\xFE\x00abc\xC4";
    assert_eq!(s.apply(input), input);
}

#[test]
fn from_system_has_current_user() {
    let user = std::env::var("USERNAME").unwrap();
    let ids = Identities::from_system(vec![]);
    assert!(ids.names.iter().any(|n| n.as_slice() == user.as_bytes()));
    assert!(ids.names.iter().all(|n| !n.is_empty()));
}
