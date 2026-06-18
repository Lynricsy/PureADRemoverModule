use crate::command_runner::validation::{validate_component, validate_simple_token};
use crate::command_runner::{AndroidCommandAdapter, CommandError, CommandInvocation, CommandPhase};

const PM: &str = "/system/bin/pm";

/// Android `pm` 组件禁用适配器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmComponentAdapter {
    user_id: u32,
    package: String,
    component: String,
}

impl PmComponentAdapter {
    /// 构造组件禁用适配器。
    pub fn new(user_id: u32, component: &str) -> Result<Self, CommandError> {
        validate_component(component)?;
        let package = package_from_component(component)?;
        Ok(Self {
            user_id,
            package,
            component: component.to_owned(),
        })
    }
}

impl AndroidCommandAdapter for PmComponentAdapter {
    fn command(&self, phase: CommandPhase) -> CommandInvocation {
        let user = self.user_id.to_string();
        match phase {
            CommandPhase::Probe => CommandInvocation::new(PM, ["path", self.package.as_str()]),
            CommandPhase::Apply => CommandInvocation::new(
                PM,
                [
                    "disable-user",
                    "--user",
                    user.as_str(),
                    self.component.as_str(),
                ],
            ),
            CommandPhase::Restore => CommandInvocation::new(
                PM,
                ["enable", "--user", user.as_str(), self.component.as_str()],
            ),
        }
    }

    fn intent(&self, phase: CommandPhase) -> String {
        format!("{} component {}", phase.as_str(), self.component)
    }
}

fn package_from_component(component: &str) -> Result<String, CommandError> {
    let Some((package, _name)) = component.split_once('/') else {
        return Err(CommandError::invalid_argument(
            "component",
            component,
            "component must include package/name",
        ));
    };
    Ok(package.to_owned())
}

pub(super) fn validate_token(field: &'static str, value: &str) -> Result<(), CommandError> {
    validate_simple_token(field, value)
}
