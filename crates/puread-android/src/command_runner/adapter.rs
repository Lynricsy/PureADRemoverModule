use crate::command_runner::{
    AndroidCommandRunner, CommandError, CommandInvocation, CommandOutput, CommandRunnerError,
};

/// Android 命令生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandPhase {
    /// 探测能力或当前状态。
    Probe,
    /// 应用期望状态。
    Apply,
    /// 恢复原状态。
    Restore,
}

impl CommandPhase {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Probe => "probe",
            Self::Apply => "apply",
            Self::Restore => "restore",
        }
    }
}

/// 命令适配层的可观察结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    phase: CommandPhase,
    intent: String,
    invocation: CommandInvocation,
    dry_run: bool,
    output: Option<CommandOutput>,
}

impl CommandOutcome {
    pub(crate) const fn dry_run(
        phase: CommandPhase,
        intent: String,
        invocation: CommandInvocation,
    ) -> Self {
        Self {
            phase,
            intent,
            invocation,
            dry_run: true,
            output: None,
        }
    }

    pub(crate) const fn executed(
        phase: CommandPhase,
        intent: String,
        invocation: CommandInvocation,
        output: CommandOutput,
    ) -> Self {
        Self {
            phase,
            intent,
            invocation,
            dry_run: false,
            output: Some(output),
        }
    }

    /// 返回生命周期阶段。
    #[must_use]
    pub const fn phase(&self) -> CommandPhase {
        self.phase
    }

    /// 返回人类可读意图。
    #[must_use]
    pub const fn intent(&self) -> &str {
        self.intent.as_str()
    }

    /// 返回命令调用。
    #[must_use]
    pub const fn invocation(&self) -> &CommandInvocation {
        &self.invocation
    }

    /// 返回是否只做 dry-run。
    #[must_use]
    pub const fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// 返回命令输出。
    #[must_use]
    pub const fn output(&self) -> Option<&CommandOutput> {
        self.output.as_ref()
    }
}

/// 支持 probe/apply/restore/dry-run 的 Android 命令适配器。
pub trait AndroidCommandAdapter {
    /// 返回指定阶段的 argv。
    fn command(&self, phase: CommandPhase) -> CommandInvocation;

    /// 返回指定阶段的人类可读意图。
    fn intent(&self, phase: CommandPhase) -> String;

    /// 探测能力或当前状态。
    fn probe<R>(&self, runner: &R) -> Result<CommandOutcome, CommandError>
    where
        R: AndroidCommandRunner,
    {
        self.run(runner, CommandPhase::Probe)
    }

    /// 应用期望状态。
    fn apply<R>(&self, runner: &R) -> Result<CommandOutcome, CommandError>
    where
        R: AndroidCommandRunner,
    {
        self.run(runner, CommandPhase::Apply)
    }

    /// 恢复原状态。
    fn restore<R>(&self, runner: &R) -> Result<CommandOutcome, CommandError>
    where
        R: AndroidCommandRunner,
    {
        self.run(runner, CommandPhase::Restore)
    }

    /// 只生成将执行的 argv 和意图，不调用 runner。
    fn dry_run(&self, phase: CommandPhase) -> CommandOutcome {
        CommandOutcome::dry_run(phase, self.intent(phase), self.command(phase))
    }

    /// 执行指定阶段命令并记录输出。
    fn run<R>(&self, runner: &R, phase: CommandPhase) -> Result<CommandOutcome, CommandError>
    where
        R: AndroidCommandRunner,
    {
        run_command(runner, phase, self.intent(phase), self.command(phase))
    }
}

fn run_command<R>(
    runner: &R,
    phase: CommandPhase,
    intent: String,
    invocation: CommandInvocation,
) -> Result<CommandOutcome, CommandError>
where
    R: AndroidCommandRunner,
{
    match runner.run(&invocation) {
        Ok(output) if output.is_success() => {
            Ok(CommandOutcome::executed(phase, intent, invocation, output))
        }
        Ok(output) => Err(CommandError::CommandFailed {
            invocation,
            status: output.status(),
            stdout: output.stdout().to_owned(),
            stderr: output.stderr().to_owned(),
        }),
        Err(
            CommandRunnerError::NotFound { detail } | CommandRunnerError::Unavailable { detail },
        ) => Err(CommandError::CommandUnavailable { invocation, detail }),
    }
}
