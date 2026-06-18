use thiserror::Error;

use crate::command_runner::CommandInvocation;

/// Android 命令适配层错误。
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandError {
    /// 命令参数不符合适配层约束。
    #[error("invalid command argument {field}: {value}: {reason}")]
    InvalidArgument {
        /// 参数字段。
        field: &'static str,
        /// 原始值。
        value: String,
        /// 拒绝原因。
        reason: &'static str,
    },
    /// ROM settings key 不在白名单内或命中禁止项。
    #[error("settings key denied: {key}: {reason}")]
    DeniedSettingsKey {
        /// Settings key。
        key: String,
        /// 拒绝原因。
        reason: &'static str,
    },
    /// 只读适配器不支持变更阶段。
    #[error("read-only adapter does not support {phase}: {intent}")]
    UnsupportedReadOnlyPhase {
        /// 被拒绝的阶段。
        phase: &'static str,
        /// 可观察意图。
        intent: String,
    },
    /// 命令无法启动。
    #[error("command unavailable: {detail}: {invocation:?}")]
    CommandUnavailable {
        /// 目标调用。
        invocation: CommandInvocation,
        /// 可观察失败详情。
        detail: String,
    },
    /// 命令以非零状态退出。
    #[error("command failed: status={status}: {invocation:?}")]
    CommandFailed {
        /// 目标调用。
        invocation: CommandInvocation,
        /// 退出码。
        status: i32,
        /// 标准输出。
        stdout: String,
        /// 标准错误。
        stderr: String,
    },
}

impl CommandError {
    pub(crate) fn invalid_argument(field: &'static str, value: &str, reason: &'static str) -> Self {
        Self::InvalidArgument {
            field,
            value: value.to_owned(),
            reason,
        }
    }

    pub(crate) fn denied_settings_key(key: &str, reason: &'static str) -> Self {
        Self::DeniedSettingsKey {
            key: key.to_owned(),
            reason,
        }
    }

    pub(crate) const fn unsupported_read_only_phase(phase: &'static str, intent: String) -> Self {
        Self::UnsupportedReadOnlyPhase { phase, intent }
    }
}
