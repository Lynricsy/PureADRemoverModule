use std::path::PathBuf;

use puread_core::restore_ledger::LedgerError;
use thiserror::Error;

use crate::sqlite_actions::types::SqliteActionSchedule;

/// `SQLite` 动作执行失败的可归类原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqliteActionFailureKind {
    /// 调度类型不允许触发 `SQLite` 动作。
    UnsupportedSchedule,
    /// 目标路径或文件类型不满足 `SQLite` 动作约束。
    InvalidTarget,
    /// 目标文件正在被其他执行者锁定或使用。
    Locked,
    /// 文件系统读写失败。
    Io,
    /// 恢复账本写入失败。
    Ledger,
    /// 目标已经移动到备份位置，但回滚恢复失败。
    Rollback,
    /// 生成的 `SQLite` 文件未通过结构校验。
    Integrity,
}

/// 批处理报告中保留的可克隆错误摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteActionFailure {
    /// 错误类别。
    pub kind: SqliteActionFailureKind,
    /// 相关路径。
    pub path: PathBuf,
    /// 可读错误说明。
    pub message: String,
}

/// `SQLite` 动作执行器内部错误。
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SqliteActionError {
    /// 不支持的调度类型。
    #[error("unsupported sqlite schedule: {schedule:?}")]
    UnsupportedSchedule {
        /// 传入调度。
        schedule: SqliteActionSchedule,
        /// 目标路径。
        path: PathBuf,
    },
    /// 目标非法。
    #[error("invalid sqlite target {path}: {reason}")]
    InvalidTarget {
        /// 目标路径。
        path: PathBuf,
        /// 拒绝原因。
        reason: &'static str,
    },
    /// 目标被锁定。
    #[error("sqlite target is locked or in use: {path}: {source}")]
    Locked {
        /// 目标路径。
        path: PathBuf,
        /// 底层锁错误。
        source: std::io::Error,
    },
    /// 文件系统读写失败。
    #[error("sqlite action I/O failed for {path}: {source}")]
    Io {
        /// 相关路径。
        path: PathBuf,
        /// 底层 I/O 错误。
        source: std::io::Error,
    },
    /// 目标可能已经被写入；此时必须保留账本和备份。
    #[error("sqlite action failed after mutation for {path}: {source}")]
    PostMutationIo {
        /// 相关路径。
        path: PathBuf,
        /// 底层 I/O 错误。
        source: std::io::Error,
    },
    /// 目标可能已经被写入；此时必须保留账本和备份。
    #[error("sqlite integrity check failed after mutation for {path}: {reason}")]
    PostMutationIntegrity {
        /// 目标路径。
        path: PathBuf,
        /// 校验失败原因。
        reason: &'static str,
    },
    /// 目标可能已经被写入；写后目标路径异常时必须保留账本和备份。
    #[error("sqlite target became invalid after mutation for {path}: {reason}")]
    PostMutationInvalidTarget {
        /// 目标路径。
        path: PathBuf,
        /// 拒绝原因。
        reason: &'static str,
    },
    /// 目标已经移动到备份位置，但回滚恢复失败；此时必须保留账本以便后续恢复。
    #[error("sqlite action rollback failed for {path}: {source}")]
    RollbackFailed {
        /// 原目标路径。
        path: PathBuf,
        /// 底层 I/O 错误。
        source: std::io::Error,
    },
    /// 账本写入失败。
    #[error("sqlite action ledger write failed for {path}: {source}")]
    Ledger {
        /// 目标路径。
        path: PathBuf,
        /// 账本错误。
        source: LedgerError,
    },
    /// `SQLite` 结构校验失败。
    #[error("minimal sqlite image is invalid for {path}: {reason}")]
    Integrity {
        /// 目标路径。
        path: PathBuf,
        /// 校验失败原因。
        reason: &'static str,
    },
}

impl SqliteActionError {
    pub(super) fn into_failure(self) -> SqliteActionFailure {
        match self {
            Self::UnsupportedSchedule { schedule, path } => SqliteActionFailure {
                kind: SqliteActionFailureKind::UnsupportedSchedule,
                path,
                message: format!("unsupported sqlite schedule: {schedule:?}"),
            },
            Self::InvalidTarget { path, reason }
            | Self::PostMutationInvalidTarget { path, reason } => SqliteActionFailure {
                kind: SqliteActionFailureKind::InvalidTarget,
                path,
                message: reason.to_owned(),
            },
            Self::Locked { path, source } => SqliteActionFailure {
                kind: SqliteActionFailureKind::Locked,
                path,
                message: source.to_string(),
            },
            Self::Io { path, source } | Self::PostMutationIo { path, source } => {
                SqliteActionFailure {
                    kind: SqliteActionFailureKind::Io,
                    path,
                    message: source.to_string(),
                }
            }
            Self::PostMutationIntegrity { path, reason } | Self::Integrity { path, reason } => {
                SqliteActionFailure {
                    kind: SqliteActionFailureKind::Integrity,
                    path,
                    message: reason.to_owned(),
                }
            }
            Self::RollbackFailed { path, source } => SqliteActionFailure {
                kind: SqliteActionFailureKind::Rollback,
                path,
                message: source.to_string(),
            },
            Self::Ledger { path, source } => SqliteActionFailure {
                kind: SqliteActionFailureKind::Ledger,
                path,
                message: source.to_string(),
            },
        }
    }

    pub(super) fn rollback_failed(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::RollbackFailed {
            path: path.into(),
            source,
        }
    }

    pub(super) const fn preserves_pending_ledger(&self) -> bool {
        matches!(
            self,
            Self::RollbackFailed { .. }
                | Self::PostMutationIo { .. }
                | Self::PostMutationIntegrity { .. }
                | Self::PostMutationInvalidTarget { .. }
        )
    }
}
