use crate::command_runner::{
    AndroidCommandAdapter, AndroidCommandRunner, CommandError, CommandInvocation, CommandOutcome,
    CommandPhase,
};

const CHCON: &str = "/system/bin/chcon";
const CHATTR: &str = "/system/bin/chattr";
const LSATTR: &str = "/system/bin/lsattr";

/// Android `chcon` 适配器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChconAdapter {
    target: String,
    apply_context: String,
    restore_context: String,
}

impl ChconAdapter {
    /// 构造 `SELinux` context 适配器。
    pub fn new(
        target: &str,
        apply_context: &str,
        restore_context: &str,
    ) -> Result<Self, CommandError> {
        validate_path(target)?;
        validate_arg("selinux_context", apply_context)?;
        validate_arg("selinux_restore_context", restore_context)?;
        Ok(Self {
            target: target.to_owned(),
            apply_context: apply_context.to_owned(),
            restore_context: restore_context.to_owned(),
        })
    }
}

impl AndroidCommandAdapter for ChconAdapter {
    fn command(&self, phase: CommandPhase) -> CommandInvocation {
        match phase {
            CommandPhase::Probe => CommandInvocation::new(CHCON, ["--help"]),
            CommandPhase::Apply => {
                CommandInvocation::new(CHCON, [self.apply_context.as_str(), self.target.as_str()])
            }
            CommandPhase::Restore => {
                CommandInvocation::new(CHCON, [self.restore_context.as_str(), self.target.as_str()])
            }
        }
    }

    fn intent(&self, phase: CommandPhase) -> String {
        format!("{} selinux context for {}", phase.as_str(), self.target)
    }
}

/// Android `chattr` 适配器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChattrAdapter {
    target: String,
}

impl ChattrAdapter {
    /// 构造 immutable 属性适配器。
    pub fn new(target: &str) -> Result<Self, CommandError> {
        validate_path(target)?;
        Ok(Self {
            target: target.to_owned(),
        })
    }
}

impl AndroidCommandAdapter for ChattrAdapter {
    fn command(&self, phase: CommandPhase) -> CommandInvocation {
        match phase {
            CommandPhase::Probe => CommandInvocation::new(CHATTR, ["--help"]),
            CommandPhase::Apply => CommandInvocation::new(CHATTR, ["+i", self.target.as_str()]),
            CommandPhase::Restore => CommandInvocation::new(CHATTR, ["-i", self.target.as_str()]),
        }
    }

    fn intent(&self, phase: CommandPhase) -> String {
        format!("{} immutable attribute for {}", phase.as_str(), self.target)
    }
}

/// Android `lsattr` 适配器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsattrAdapter {
    target: String,
}

impl LsattrAdapter {
    /// 构造属性读取适配器。
    pub fn new(target: &str) -> Result<Self, CommandError> {
        validate_path(target)?;
        Ok(Self {
            target: target.to_owned(),
        })
    }
}

impl AndroidCommandAdapter for LsattrAdapter {
    fn command(&self, _phase: CommandPhase) -> CommandInvocation {
        CommandInvocation::new(LSATTR, [self.target.as_str()])
    }

    fn intent(&self, phase: CommandPhase) -> String {
        format!(
            "{} immutable attributes for {}",
            phase.as_str(),
            self.target
        )
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

fn validate_path(path: &str) -> Result<(), CommandError> {
    if path.is_empty() || path.contains('\0') || path.starts_with('-') {
        return Err(CommandError::invalid_argument(
            "target",
            path,
            "path must be non-empty, contain no NUL, and not look like a command option",
        ));
    }
    Ok(())
}

fn validate_arg(field: &'static str, value: &str) -> Result<(), CommandError> {
    if value.is_empty() || value.contains(char::is_whitespace) || value.contains('\0') {
        return Err(CommandError::invalid_argument(
            field,
            value,
            "argument must be non-empty and contain no whitespace",
        ));
    }
    Ok(())
}
