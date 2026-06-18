use crate::command_runner::pm::validate_token;
use crate::command_runner::{AndroidCommandAdapter, CommandError, CommandInvocation, CommandPhase};

const SETTINGS: &str = "/system/bin/settings";
const ALLOWED_ROM_SETTINGS: &[&str] = &[
    "miui_home_show_recommend",
    "miui_home_personalized",
    "miui_personalized_ad_enabled",
    "miui_system_ad_solution",
    "com.miui.systemAdSolution.adSwitch",
];

/// Android settings namespace。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SettingsNamespace {
    /// `settings system`。
    System,
    /// `settings secure`。
    Secure,
    /// `settings global`。
    Global,
}

impl SettingsNamespace {
    const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Secure => "secure",
            Self::Global => "global",
        }
    }
}

/// Android `settings` ROM profile 适配器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsAdapter {
    namespace: SettingsNamespace,
    key: String,
    apply_value: String,
    restore_value: Option<String>,
}

impl SettingsAdapter {
    /// 构造白名单 ROM profile settings 适配器。
    pub fn new(
        namespace: SettingsNamespace,
        key: &str,
        apply_value: &str,
        restore_value: Option<&str>,
    ) -> Result<Self, CommandError> {
        validate_settings_key(key)?;
        validate_value("settings_value", apply_value)?;
        if let Some(value) = restore_value {
            validate_value("settings_restore_value", value)?;
        }
        Ok(Self {
            namespace,
            key: key.to_owned(),
            apply_value: apply_value.to_owned(),
            restore_value: restore_value.map(str::to_owned),
        })
    }
}

impl AndroidCommandAdapter for SettingsAdapter {
    fn command(&self, phase: CommandPhase) -> CommandInvocation {
        let namespace = self.namespace.as_str();
        match phase {
            CommandPhase::Probe => {
                CommandInvocation::new(SETTINGS, ["get", namespace, self.key.as_str()])
            }
            CommandPhase::Apply => CommandInvocation::new(
                SETTINGS,
                [
                    "put",
                    namespace,
                    self.key.as_str(),
                    self.apply_value.as_str(),
                ],
            ),
            CommandPhase::Restore => self.restore_command(namespace),
        }
    }

    fn intent(&self, phase: CommandPhase) -> String {
        format!("{} setting {}", phase.as_str(), self.key)
    }
}

impl SettingsAdapter {
    fn restore_command(&self, namespace: &str) -> CommandInvocation {
        if let Some(value) = self.restore_value.as_deref() {
            return CommandInvocation::new(SETTINGS, ["put", namespace, self.key.as_str(), value]);
        }
        CommandInvocation::new(SETTINGS, ["delete", namespace, self.key.as_str()])
    }
}

fn validate_settings_key(key: &str) -> Result<(), CommandError> {
    validate_token("settings_key", key)?;
    let lower = key.to_ascii_lowercase();
    if key == "private_dns_mode" || key == "private_dns_specifier" || lower.contains("dns") {
        return Err(CommandError::denied_settings_key(
            key,
            "DNS settings are out of scope",
        ));
    }
    if lower.contains("host") {
        return Err(CommandError::denied_settings_key(
            key,
            "hosts settings are out of scope",
        ));
    }
    if lower.contains("proxy") {
        return Err(CommandError::denied_settings_key(
            key,
            "proxy settings are out of scope",
        ));
    }
    if ALLOWED_ROM_SETTINGS.contains(&key) {
        return Ok(());
    }
    Err(CommandError::denied_settings_key(
        key,
        "settings key is not in the ROM profile allowlist",
    ))
}

fn validate_value(field: &'static str, value: &str) -> Result<(), CommandError> {
    if value.contains('\0') || value.contains('\n') {
        return Err(CommandError::invalid_argument(
            field,
            value,
            "value must not contain NUL or newline",
        ));
    }
    Ok(())
}
