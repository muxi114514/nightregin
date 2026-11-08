//! PARAM 参数表：regulation 里的文件和游戏内存中加载后的数据是同一种格式，解析代码两边共用。

mod file;
pub mod layout;

pub use file::{DESCRIPTOR_SIZE, HEADER_SIZE, ParamFile, Row, field};
