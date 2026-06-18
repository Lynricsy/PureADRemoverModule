use crate::command_runner::validation::validate_property;
use crate::command_runner::{
    AndroidCommandAdapter, AndroidCommandRunner, CommandError, CommandInvocation, CommandOutcome,
    CommandPhase,
};

const GETPROP: &str = "/system/bin/getprop";

/// Android `getprop` 只读属性适配器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetpropAdapter {
    property: String,
}

impl GetpropAdapter {
    /// 构造属性读取适配器。
    pub fn new(property: &str) -> Result<Self, CommandError> {
        validate_property(property)?;
        Ok(Self {
            property: property.to_owned(),
        })
    }
}

impl AndroidCommandAdapter for GetpropAdapter {
    fn command(&self, _phase: CommandPhase) -> CommandInvocation {
        CommandInvocation::new(GETPROP, [self.property.as_str()])
    }

    fn intent(&self, phase: CommandPhase) -> String {
        format!("{} property {}", phase.as_str(), self.property)
    }

    fn apply<R>(&self, _runner: &R) -> Result<CommandOutcome, CommandError>
    where
        R: AndroidCommandRunner,
    {
        Err(CommandError::unsupported_read_only_phase(
            CommandPhase::Apply.as_str(),
            self.intent(CommandPhase::Apply),
        ))
    }

    fn restore<R>(&self, _runner: &R) -> Result<CommandOutcome, CommandError>
    where
        R: AndroidCommandRunner,
    {
        Err(CommandError::unsupported_read_only_phase(
            CommandPhase::Restore.as_str(),
            self.intent(CommandPhase::Restore),
        ))
    }
}
