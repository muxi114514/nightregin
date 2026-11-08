//! INI（nrsc_settings.ini、ChanceSetter.ini）：按行保存原始字节，to_bytes 逐字节还原；
//! set 只替换目标键的值，同一行的空白、对齐和行尾注释都不动

use std::path::Path;

use crate::profile::ConfigError;
use crate::text::UTF8_BOM;

/// 一份 INI 文档：行内容不改空白、大小写和编码
#[derive(Clone, Debug, PartialEq)]
pub struct IniDoc {
    /// 去掉换行后的各行
    lines: Vec<Vec<u8>>,
    /// 行间分隔：每个换行都是 CRLF（含没有换行）时为 "\r\n"，行内不留 \r；否则为 "\n"，\r 留在行内
    newline: &'static [u8],
    bom: bool,
    /// 最后一行后面有换行
    trailing_newline: bool,
}

/// set 找不到目标键
#[derive(Debug, thiserror::Error)]
#[error("配置里缺少 [{section}] {key}")]
pub struct IniMissing {
    pub section: String,
    pub key: String,
}

/// 一行的含义；名字和值都已去掉两端空白
enum Line<'a> {
    Section(&'a [u8]),
    Entry {
        key: &'a [u8],
        value: &'a [u8],
    },
    /// 空行、注释行、无法识别的行
    Other,
}

impl IniDoc {
    /// 任何字节都能解析，不会失败
    pub fn parse(bytes: &[u8]) -> IniDoc {
        let (bom, body) = match bytes.strip_prefix(UTF8_BOM) {
            Some(rest) => (true, rest),
            None => (false, bytes),
        };
        let mut lines: Vec<Vec<u8>> = body.split(|&b| b == b'\n').map(<[u8]>::to_vec).collect();
        // split 至少产出一段；最后一段后面没有 \n，为空说明原文以换行结尾（或是空文件）
        let last = lines.pop().unwrap_or_default();
        let crlf = lines.iter().all(|line| line.ends_with(b"\r"));
        if crlf {
            for line in &mut lines {
                line.pop();
            }
        }
        if !last.is_empty() {
            lines.push(last);
        }
        IniDoc {
            lines,
            newline: if crlf { b"\r\n" } else { b"\n" },
            bom,
            trailing_newline: body.ends_with(b"\n"),
        }
    }

    /// 节名、键名不区分大小写；section 为 "" 时查第一个节之前的键；重复的节只看第一个，重复的键取第一个。
    /// 非 UTF-8 的字节换成替换符
    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let i = self.find(section, key)?;
        match classify(&self.lines[i]) {
            Line::Entry { value, .. } => Some(String::from_utf8_lossy(value).into_owned()),
            _ => None,
        }
    }

    /// 替换目标键的值，查找规则同 get；值变了才返回 true，找不到键时什么都不改
    pub fn set(&mut self, section: &str, key: &str, value: &str) -> Result<bool, IniMissing> {
        let i = self.find(section, key).ok_or_else(|| IniMissing {
            section: section.to_owned(),
            key: key.to_owned(),
        })?;
        let line = &self.lines[i];
        // find 找到的一定是键值行，第一个 = 就是 classify 用的那个
        let Some(eq) = line.iter().position(|&b| b == b'=') else {
            return Ok(false);
        };
        let start = eq + 1;
        let raw = &line[start..];
        let body = strip_comment(raw);
        let current = body.trim_ascii();
        if current == value.as_bytes() {
            return Ok(false);
        }
        let mut out = line[..start].to_vec();
        if current.is_empty() {
            out.push(b' ');
            out.extend_from_slice(value.as_bytes());
            // 混合换行时行尾的 \r 留在行内，不能吃掉
            if body.ends_with(b"\r") {
                out.push(b'\r');
            }
        } else {
            let lead = body.len() - body.trim_ascii_start().len();
            out.extend_from_slice(&body[..lead]);
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(&body[lead + current.len()..]);
        }
        out.extend_from_slice(&raw[body.len()..]);
        self.lines[i] = out;
        Ok(true)
    }

    /// 目标键所在的行：离开第一个同名节就不再找
    fn find(&self, section: &str, key: &str) -> Option<usize> {
        let mut in_section = section.is_empty();
        for (i, line) in self.lines.iter().enumerate() {
            match classify(line) {
                Line::Section(name) => {
                    if in_section {
                        return None;
                    }
                    in_section = name.eq_ignore_ascii_case(section.as_bytes());
                }
                Line::Entry { key: k, .. }
                    if in_section && k.eq_ignore_ascii_case(key.as_bytes()) =>
                {
                    return Some(i);
                }
                _ => {}
            }
        }
        None
    }

    /// 与 parse 的输入逐字节相同
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.bom {
            out.extend_from_slice(UTF8_BOM);
        }
        out.extend(self.lines.join(self.newline));
        if self.trailing_newline {
            out.extend_from_slice(self.newline);
        }
        out
    }
}

/// 只有读文件会失败（Io），内容总能解析
pub fn read_ini(path: &Path) -> Result<IniDoc, ConfigError> {
    Ok(IniDoc::parse(&std::fs::read(path)?))
}

/// 以 ; 或 # 开头的是注释行；[名字] 是节头，] 之后的内容忽略；其余含 = 的行是键值
fn classify(line: &[u8]) -> Line<'_> {
    let text = line.trim_ascii();
    if matches!(text.first(), None | Some(b';' | b'#')) {
        return Line::Other;
    }
    if let Some(rest) = text.strip_prefix(b"[")
        && let Some(end) = rest.iter().position(|&b| b == b']')
    {
        return Line::Section(rest[..end].trim_ascii());
    }
    match text.iter().position(|&b| b == b'=') {
        Some(eq) => Line::Entry {
            key: text[..eq].trim_ascii(),
            value: strip_comment(&text[eq + 1..]).trim_ascii(),
        },
        None => Line::Other,
    }
}

/// 去掉值里「空白之后的 ; 或 #」开始的注释；紧挨着的（如 C:\a;b）不算
fn strip_comment(raw: &[u8]) -> &[u8] {
    match raw
        .windows(2)
        .position(|w| w[0].is_ascii_whitespace() && matches!(w[1], b';' | b'#'))
    {
        Some(space) => &raw[..space],
        None => raw,
    }
}

#[cfg(test)]
mod tests {
    use super::IniDoc;

    /// 行模型是计划 2 写回的基础，公开接口看不出来，在这里单独钉住
    #[test]
    fn ini_line_model() {
        let crlf = IniDoc::parse(b"a\r\nb\r\n");
        assert_eq!(crlf.lines, [b"a".to_vec(), b"b".to_vec()]);
        assert_eq!(crlf.newline, b"\r\n");
        let mixed = IniDoc::parse(b"a\r\nb\n");
        assert_eq!(mixed.lines, [b"a\r".to_vec(), b"b".to_vec()]);
        assert_eq!(mixed.newline, b"\n");
        assert_eq!(IniDoc::parse(b"a").newline, b"\r\n");
        assert!(IniDoc::parse(b"").lines.is_empty());
    }
}
