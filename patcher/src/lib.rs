//! regulation.bin 的容器读写（AES → DCX/ZSTD → BND4 → PARAM）与 NRStatAlloc 组件补丁。
//! 补丁工具 exe 与整合包重平衡工具（rebalance/）共用。

pub mod bnd4;
mod components;
pub mod crypto;
pub mod dcx;
pub mod param_write;
pub mod regulation;

#[cfg(test)]
mod tests;
