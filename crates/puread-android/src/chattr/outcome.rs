use crate::chattr::command::CommandInvocation;
use crate::chattr::error::ImmutableError;

/// 已执行命令摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedCommand {
    program: String,
    args: Vec<String>,
    status: i32,
}

impl ObservedCommand {
    pub(crate) fn from_invocation(invocation: &CommandInvocation, status: i32) -> Self {
        Self {
            program: invocation.program().to_owned(),
            args: invocation.args().to_vec(),
            status,
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

    /// 返回退出码。
    #[must_use]
    pub const fn status(&self) -> i32 {
        self.status
    }
}

/// immutable 尝试的可观察结果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImmutableOutcome {
    /// 非强力 profile 只产生 skip/plan。
    Skipped {
        /// 目标路径。
        target: String,
        /// 跳过原因。
        reason: String,
    },
    /// 强力 profile 成功应用 immutable。
    Applied {
        /// 目标路径。
        target: String,
        /// 执行前属性。
        original_attrs: Option<String>,
        /// 已执行命令。
        command: ObservedCommand,
    },
    /// 能力不可用或命令失败，降级为可观察结果。
    Degraded {
        /// 目标路径。
        target: String,
        /// 执行前属性。
        original_attrs: Option<String>,
        /// 降级原因。
        reason: &'static str,
        /// 原始错误。
        error: ImmutableError,
    },
}
