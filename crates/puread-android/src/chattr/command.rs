use crate::chattr::contains_failure_marker;
use crate::chattr::error::ImmutableError;
use crate::command_runner::{
    AndroidCommandRunner, CommandInvocation as AndroidCommandInvocation, CommandRunnerError,
    RealAndroidCommandRunner,
};

/// 可观察的命令调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    program: String,
    args: Vec<String>,
}

impl CommandInvocation {
    /// 构造命令调用描述。
    #[must_use]
    pub fn new<I, S>(program: &str, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            program: program.to_owned(),
            args: args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect(),
        }
    }

    /// 返回命令名。
    #[must_use]
    pub const fn program(&self) -> &str {
        self.program.as_str()
    }

    /// 返回命令参数。
    #[must_use]
    pub const fn args(&self) -> &[String] {
        self.args.as_slice()
    }
}

/// 命令执行输出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

impl CommandOutput {
    /// 构造成功输出。
    #[must_use]
    pub fn success(stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self {
            status: 0,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    /// 构造失败输出。
    #[must_use]
    pub fn failure(status: i32, stderr: impl Into<String>, stdout: impl Into<String>) -> Self {
        Self {
            status,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    /// 返回进程退出码。
    #[must_use]
    pub const fn status(&self) -> i32 {
        self.status
    }

    /// 返回标准输出。
    #[must_use]
    pub const fn stdout(&self) -> &str {
        self.stdout.as_str()
    }

    /// 返回标准错误。
    #[must_use]
    pub const fn stderr(&self) -> &str {
        self.stderr.as_str()
    }

    pub(crate) fn is_success_without_failure_marker(&self) -> bool {
        self.status == 0 && !contains_failure_marker(&self.stdout, &self.stderr)
    }
}

/// 命令执行边界。
pub trait CommandRunner {
    /// 执行一次命令调用。
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, ImmutableError>;
}

/// 使用系统 PATH 的真实命令执行器。
#[derive(Debug, Clone, Copy, Default)]
pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, ImmutableError> {
        let android_invocation =
            AndroidCommandInvocation::new(invocation.program(), invocation.args());
        let output = RealAndroidCommandRunner
            .run(&android_invocation)
            .map_err(|source| immutable_runner_error(invocation, source))?;
        Ok(CommandOutput {
            status: output.status(),
            stdout: output.stdout().to_owned(),
            stderr: output.stderr().to_owned(),
        })
    }
}

fn immutable_runner_error(
    invocation: &CommandInvocation,
    source: CommandRunnerError,
) -> ImmutableError {
    let detail = match source {
        CommandRunnerError::NotFound { detail } | CommandRunnerError::Unavailable { detail } => {
            detail
        }
    };
    ImmutableError::CommandUnavailable {
        program: invocation.program().to_owned(),
        detail,
    }
}
