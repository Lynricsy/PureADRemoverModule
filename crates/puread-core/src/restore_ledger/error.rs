use std::path::PathBuf;

use thiserror::Error;

/// 状态账本错误。
#[derive(Debug, Error)]
pub enum LedgerError {
    /// 文件系统读写失败。
    #[error("ledger I/O failed for {path}: {source}")]
    Io {
        /// 账本路径。
        path: PathBuf,
        /// 底层 I/O 错误。
        source: std::io::Error,
    },
    /// JSONL 某一行无法反序列化。
    #[error("ledger JSONL line {line} is invalid: {source}")]
    JsonLine {
        /// 1-based 行号。
        line: usize,
        /// JSON 解析错误。
        source: serde_json::Error,
    },
    /// 账本记录字段不满足模型约束。
    #[error("ledger field {field} is invalid: {reason}")]
    InvalidRecord {
        /// 字段名。
        field: &'static str,
        /// 字段值。
        value: String,
        /// 拒绝原因。
        reason: &'static str,
    },
    /// JSON 序列化失败。
    #[error("ledger JSON serialization failed: {source}")]
    JsonWrite {
        /// JSON 序列化错误。
        source: serde_json::Error,
    },
}
