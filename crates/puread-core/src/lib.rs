#![doc = "`PureAD` 核心类型与规则模型。"]

/// 规则模型错误。
pub mod error;
/// 类型化规则模型。
pub mod model;

#[cfg(feature = "path-expansion")]
/// Android 路径展开模型。
pub mod path_expansion;

#[cfg(feature = "restore-ledger")]
/// 可恢复状态账本模型。
pub mod restore_ledger;
