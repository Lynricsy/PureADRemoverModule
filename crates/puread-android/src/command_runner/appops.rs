use crate::command_runner::validation::{validate_appop, validate_package};
use crate::command_runner::{AndroidCommandAdapter, CommandError, CommandInvocation, CommandPhase};

const CMD: &str = "/system/bin/cmd";

/// Android `cmd appops` 适配器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppOpsAdapter {
    package: String,
    op: String,
    apply_mode: String,
    restore_mode: String,
}

impl AppOpsAdapter {
    /// 构造 `AppOps` 设置适配器。
    pub fn new(
        package: &str,
        op: &str,
        apply_mode: &str,
        restore_mode: &str,
    ) -> Result<Self, CommandError> {
        validate_package("package", package)?;
        validate_appop(op)?;
        validate_appop_mode(apply_mode)?;
        validate_appop_mode(restore_mode)?;
        Ok(Self {
            package: package.to_owned(),
            op: op.to_owned(),
            apply_mode: apply_mode.to_owned(),
            restore_mode: restore_mode.to_owned(),
        })
    }
}

impl AndroidCommandAdapter for AppOpsAdapter {
    fn command(&self, phase: CommandPhase) -> CommandInvocation {
        match phase {
            CommandPhase::Probe => CommandInvocation::new(
                CMD,
                ["appops", "get", self.package.as_str(), self.op.as_str()],
            ),
            CommandPhase::Apply => self.set_command(self.apply_mode.as_str()),
            CommandPhase::Restore => self.set_command(self.restore_mode.as_str()),
        }
    }

    fn intent(&self, phase: CommandPhase) -> String {
        format!("{} appop {} for {}", phase.as_str(), self.op, self.package)
    }
}

impl AppOpsAdapter {
    fn set_command(&self, mode: &str) -> CommandInvocation {
        CommandInvocation::new(
            CMD,
            [
                "appops",
                "set",
                self.package.as_str(),
                self.op.as_str(),
                mode,
            ],
        )
    }
}

fn validate_appop_mode(mode: &str) -> Result<(), CommandError> {
    match mode {
        "allow" | "foreground" | "ignore" | "deny" | "default" => Ok(()),
        _ => Err(CommandError::invalid_argument(
            "appop_mode",
            mode,
            "unsupported appop mode",
        )),
    }
}
