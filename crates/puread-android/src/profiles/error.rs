use std::path::PathBuf;

use thiserror::Error;

use crate::command_runner::CommandError;

/// Android profile 执行错误。
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ProfileError {
    /// 命令适配层拒绝或执行失败。
    #[error("profile command failed: {0}")]
    Command(#[from] CommandError),
    /// 命令启动失败。
    #[error("profile command runner failed: {detail}")]
    Runner {
        /// 可观察错误详情。
        detail: String,
    },
    /// 文件系统读写失败。
    #[error("profile file I/O failed for {path}: {source}")]
    Io {
        /// 目标路径。
        path: PathBuf,
        /// 底层错误。
        source: std::io::Error,
    },
    /// XML 解析或写入失败。
    #[error("profile XML mutation failed for {path}: {reason}")]
    Xml {
        /// 目标路径。
        path: PathBuf,
        /// 可观察失败原因。
        reason: String,
    },
    /// JSON 记录序列化或反序列化失败。
    #[error("profile record JSON failed: {source}")]
    Json {
        /// JSON 错误。
        source: serde_json::Error,
    },
    /// 规则字段不满足 profile 边界。
    #[error("invalid profile field {field}: {value}: {reason}")]
    InvalidRule {
        /// 字段名。
        field: &'static str,
        /// 字段值。
        value: String,
        /// 拒绝原因。
        reason: &'static str,
    },
}

impl ProfileError {
    pub(crate) fn invalid_rule(field: &'static str, value: &str, reason: &'static str) -> Self {
        Self::InvalidRule {
            field,
            value: value.to_owned(),
            reason,
        }
    }

    pub(crate) const fn io(path: PathBuf, source: std::io::Error) -> Self {
        Self::Io { path, source }
    }

    pub(crate) const fn json(source: serde_json::Error) -> Self {
        Self::Json { source }
    }

    pub(crate) fn xml(path: PathBuf, reason: impl Into<String>) -> Self {
        Self::Xml {
            path,
            reason: reason.into(),
        }
    }
}
