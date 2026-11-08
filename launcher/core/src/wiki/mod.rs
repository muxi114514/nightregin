//! 图鉴：launcher\data\wiki.json 原样交给前端（不认识的字段由前端跳过），图片只开放 img\*.webp

pub mod odds;

use std::fs;
use std::io;

use serde::Serialize;
use serde_json::Value;

use crate::manifest::read_manifest_info;
use crate::paths::PackLayout;
use crate::retry;
use crate::text::strip_utf8_bom;

/// 结构变了才升；加版式、加字段靠前端跳过不认识的内容
pub const WIKI_FORMAT: u64 = 1;

const WIKI_FILE: &str = "wiki.json";
const IMAGE_DIR: &str = "img";
const IMAGE_PREFIX: &str = "img/";
const IMAGE_SUFFIX: &str = ".webp";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum WikiView {
    Ok {
        data: Value,
        #[serde(rename = "newIds")]
        new_ids: Vec<String>,
    },
    Missing,
    Incompatible {
        format: u64,
    },
    Broken {
        message: String,
    },
}

pub fn read_wiki(pack: &PackLayout) -> WikiView {
    let path = pack.wiki_dir().join(WIKI_FILE);
    let bytes = match retry::transient(|| fs::read(&path)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return WikiView::Missing,
        Err(e) => return broken(e.to_string()),
    };
    let data: Value = match serde_json::from_slice(strip_utf8_bom(&bytes)) {
        Ok(data) => data,
        Err(e) => return broken(e.to_string()),
    };
    let Some(format) = data.get("format").and_then(Value::as_u64) else {
        return broken("缺少 format".to_owned());
    };
    if format > WIKI_FORMAT {
        return WikiView::Incompatible { format };
    }
    let has_categories = data
        .get("categories")
        .and_then(Value::as_array)
        .is_some_and(|list| !list.is_empty());
    if !has_categories {
        return broken("缺少 categories".to_owned());
    }
    WikiView::Ok {
        data,
        new_ids: new_ids(pack),
    }
}

fn broken(message: String) -> WikiView {
    log::warn!("图鉴数据损坏：{message}");
    WikiView::Broken { message }
}

/// 「本版新增」只认本地 manifest 的 notes，更新页和图鉴不会互相矛盾
fn new_ids(pack: &PackLayout) -> Vec<String> {
    read_manifest_info(&pack.manifest())
        .map(|info| info.notes.into_iter().filter_map(|n| n.wiki).collect())
        .unwrap_or_default()
}

/// rel 只接受 img/<字母数字下划线连字符>.webp，其余一律当不存在
pub fn read_data_image(pack: &PackLayout, rel: &str) -> Option<Vec<u8>> {
    let stem = image_stem(rel)?;
    let path = pack
        .wiki_dir()
        .join(IMAGE_DIR)
        .join(format!("{stem}{IMAGE_SUFFIX}"));
    retry::transient(|| fs::read(&path)).ok()
}

fn image_stem(rel: &str) -> Option<&str> {
    let stem = rel.strip_prefix(IMAGE_PREFIX)?.strip_suffix(IMAGE_SUFFIX)?;
    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    (!stem.is_empty() && stem.chars().all(allowed)).then_some(stem)
}
