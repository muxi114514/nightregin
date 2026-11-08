//! 图鉴数据：wiki.json 的读取、本版新增、图片路径校验

mod common;

use std::fs;

use common::{FakePack, fake_pack};
use launcher_core::paths::PackLayout;
use launcher_core::wiki::{WikiView, read_data_image, read_wiki};
use serde_json::json;

fn setup() -> (FakePack, PackLayout) {
    let fp = fake_pack("pack");
    let pack = PackLayout {
        root: fp.pack_root.clone(),
    };
    (fp, pack)
}

fn write_wiki(pack: &PackLayout, body: &[u8]) {
    fs::write(pack.wiki_dir().join("wiki.json"), body).unwrap();
}

fn write_manifest(pack: &PackLayout, body: &str) {
    fs::write(pack.manifest(), body).unwrap();
}

#[test]
fn wiki_read_ok_returns_data_and_new_ids() {
    let (_fp, pack) = setup();
    write_manifest(
        &pack,
        r#"{"notes":[{"text":"新武器","wiki":"weapon-1"},{"text":"修了个问题"}]}"#,
    );
    let WikiView::Ok { data, new_ids } = read_wiki(&pack) else {
        panic!("应该读取成功");
    };
    assert_eq!(data["format"], 1);
    assert_eq!(new_ids, ["weapon-1"]);
}

#[test]
fn wiki_no_manifest_means_no_new_ids() {
    let (_fp, pack) = setup();
    let WikiView::Ok { new_ids, .. } = read_wiki(&pack) else {
        panic!("应该读取成功");
    };
    assert!(new_ids.is_empty());
}

#[test]
fn wiki_missing_file() {
    let (_fp, pack) = setup();
    fs::remove_file(pack.wiki_dir().join("wiki.json")).unwrap();
    assert_eq!(read_wiki(&pack), WikiView::Missing);
}

#[test]
fn wiki_utf8_bom_ok() {
    let (_fp, pack) = setup();
    write_wiki(
        &pack,
        b"\xEF\xBB\xBF{\"format\":1,\"categories\":[{\"id\":\"a\",\"name\":\"a\"}]}",
    );
    assert!(matches!(read_wiki(&pack), WikiView::Ok { .. }));
}

#[test]
fn wiki_bad_json_is_broken() {
    let (_fp, pack) = setup();
    write_wiki(&pack, b"{not json");
    let WikiView::Broken { message } = read_wiki(&pack) else {
        panic!("应该是损坏");
    };
    assert!(!message.is_empty());
}

#[test]
fn wiki_no_format_is_broken() {
    let (_fp, pack) = setup();
    write_wiki(&pack, br#"{"categories":[]}"#);
    assert!(matches!(read_wiki(&pack), WikiView::Broken { .. }));
}

#[test]
fn wiki_no_categories_is_broken() {
    let (_fp, pack) = setup();
    write_wiki(&pack, br#"{"format":1}"#);
    assert!(matches!(read_wiki(&pack), WikiView::Broken { .. }));
}

#[test]
fn wiki_empty_categories_is_broken() {
    let (_fp, pack) = setup();
    write_wiki(&pack, br#"{"format":1,"categories":[]}"#);
    assert!(matches!(read_wiki(&pack), WikiView::Broken { .. }));
}

#[test]
fn wiki_format_too_new() {
    let (_fp, pack) = setup();
    write_wiki(&pack, br#"{"format":2,"categories":[]}"#);
    assert_eq!(read_wiki(&pack), WikiView::Incompatible { format: 2 });
}

#[test]
fn wiki_unknown_fields_pass_through() {
    let (_fp, pack) = setup();
    write_wiki(
        &pack,
        br#"{"format":1,"categories":[{"id":"x","name":"x","entries":[{"id":"a","name":"a","foo":7}]}]}"#,
    );
    let WikiView::Ok { data, .. } = read_wiki(&pack) else {
        panic!("应该读取成功");
    };
    assert_eq!(data["categories"][0]["entries"][0]["foo"], 7);
}

#[test]
fn wiki_image_reads_webp() {
    let (_fp, pack) = setup();
    let expected = fs::read(pack.wiki_dir().join("img").join("w_1.webp")).unwrap();
    assert_eq!(read_data_image(&pack, "img/w_1.webp"), Some(expected));
}

#[test]
fn wiki_image_missing_is_none() {
    let (_fp, pack) = setup();
    assert_eq!(read_data_image(&pack, "img/none.webp"), None);
}

#[test]
fn wiki_image_rejects_bad_paths() {
    let (_fp, pack) = setup();
    // 这些路径就算指向真实文件也不能读出来
    fs::write(pack.launcher_dir().join("a.webp"), b"secret").unwrap();
    for bad in [
        "../settings.json",
        "../a.webp",
        "img/../../a.webp",
        "img/../w_1.webp",
        "img/a/b.webp",
        "img\\a.webp",
        "C:/x.webp",
        "/img/w_1.webp",
        "img/w_1.png",
        "img/.webp",
        "img/w_1.webp:ads",
        "",
    ] {
        assert_eq!(read_data_image(&pack, bad), None, "{bad}");
    }
}

#[test]
fn wiki_view_json_shape() {
    let shape = |v: &WikiView| serde_json::to_value(v).unwrap();
    assert_eq!(
        shape(&WikiView::Ok {
            data: json!({"format": 1}),
            new_ids: vec!["a".into()],
        }),
        json!({"state": "ok", "data": {"format": 1}, "newIds": ["a"]})
    );
    assert_eq!(shape(&WikiView::Missing), json!({"state": "missing"}));
    assert_eq!(
        shape(&WikiView::Incompatible { format: 2 }),
        json!({"state": "incompatible", "format": 2})
    );
    assert_eq!(
        shape(&WikiView::Broken {
            message: "x".into()
        }),
        json!({"state": "broken", "message": "x"})
    );
}
