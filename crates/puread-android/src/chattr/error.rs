use thiserror::Error;

/// immutable 适配层错误。
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImmutableError {
    /// 命令不存在或无法启动。
    #[error("command unavailable: {program}: {detail}")]
    CommandUnavailable {
        /// 命令名。
        program: String,
        /// 可观察错误详情。
        detail: String,
    },
    /// 命令返回失败或输出包含失败标记。
    #[error("command failed: {program} status={status}")]
    CommandFailed {
        /// 命令名。
        program: String,
        /// 命令参数。
        args: Vec<String>,
        /// 退出码。
        status: i32,
        /// 标准输出。
        stdout: String,
        /// 标准错误。
        stderr: String,
    },
    /// `lsattr` 输出无法解析。
    #[error("malformed lsattr output: {stdout}")]
    MalformedLsattr {
        /// 原始标准输出。
        stdout: String,
    },
    /// 测试 runner 收到未脚本化命令。
    #[error("unscripted command in test runner: {program} {args:?}")]
    UnscriptedCommand {
        /// 命令名。
        program: String,
        /// 命令参数。
        args: Vec<String>,
    },
}
