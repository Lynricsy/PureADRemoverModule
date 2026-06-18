use std::io::ErrorKind;
use std::process::Command;

use thiserror::Error;

use crate::command_runner::{CommandInvocation, CommandOutput};

/// 命令启动错误。
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandRunnerError {
    /// 命令路径不存在。
    #[error("command not found: {detail}")]
    NotFound {
        /// 可观察错误详情。
        detail: String,
    },
    /// 命令无法启动。
    #[error("command unavailable: {detail}")]
    Unavailable {
        /// 可观察错误详情。
        detail: String,
    },
}

/// Android 命令执行边界。
pub trait AndroidCommandRunner {
    /// 执行一次 argv 调用。
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError>;
}

/// 真实 Android 命令执行器。
#[derive(Debug, Clone, Copy, Default)]
pub struct RealAndroidCommandRunner;

impl AndroidCommandRunner for RealAndroidCommandRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
        let output = Command::new(invocation.program())
            .args(invocation.args())
            .output()
            .map_err(|source| command_start_error(&source))?;
        let status = output.status.code().map_or(1, |code| code);
        Ok(CommandOutput::from_status(
            status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

fn command_start_error(source: &std::io::Error) -> CommandRunnerError {
    if source.kind() == ErrorKind::NotFound {
        return CommandRunnerError::NotFound {
            detail: source.to_string(),
        };
    }
    CommandRunnerError::Unavailable {
        detail: source.to_string(),
    }
}
