use std::path::PathBuf;

use puread_core::restore_ledger::LedgerError;
use thiserror::Error;

/// 文件动作规划或执行错误。
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FileActionError {
    /// 目标路径不满足安全边界。
    #[error("rejected file action target {path}: {reason}")]
    RejectedTarget {
        /// 被拒绝路径。
        path: PathBuf,
        /// 拒绝原因。
        reason: &'static str,
    },
    /// 文件系统读写失败。
    #[error("file action I/O failed for {path}: {source}")]
    Io {
        /// 失败路径。
        path: PathBuf,
        /// 底层 I/O 错误。
        source: std::io::Error,
    },
    /// 目标已经移动到备份位置，但回滚恢复失败；此时必须保留账本以便后续恢复。
    #[error("file action rollback failed for {path}: {source}")]
    RollbackFailed {
        /// 原目标路径。
        path: PathBuf,
        /// 底层 I/O 错误。
        source: std::io::Error,
    },
    /// 账本读写失败。
    #[error("file action ledger failed: {source}")]
    Ledger {
        /// 底层账本错误。
        #[from]
        source: LedgerError,
    },
    /// 元数据封装失败。
    #[error("metadata operation {operation} failed for {path}: {detail}")]
    Metadata {
        /// 元数据操作名。
        operation: &'static str,
        /// 目标路径。
        path: PathBuf,
        /// 失败详情。
        detail: String,
    },
}

impl FileActionError {
    pub(super) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub(super) fn rejected_target(path: impl Into<PathBuf>, reason: &'static str) -> Self {
        Self::RejectedTarget {
            path: path.into(),
            reason,
        }
    }

    pub(super) fn rollback_failed(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::RollbackFailed {
            path: path.into(),
            source,
        }
    }

    pub(super) const fn preserves_pending_ledger(&self) -> bool {
        matches!(self, Self::RollbackFailed { .. })
    }
}
