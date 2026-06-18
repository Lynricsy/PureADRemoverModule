use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::command_runner::{
    AppOpsAdapter, CommandError, PmComponentAdapter, SettingsAdapter, SettingsNamespace,
};
use crate::profiles::ProfileError;

/// `AppOps` profile 规则。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppOpProfileRule {
    pub(crate) id: String,
    pub(crate) package: String,
    pub(crate) op: String,
    pub(crate) mode: String,
}

impl AppOpProfileRule {
    /// 构造 `AppOps` profile 规则。
    pub fn new(id: &str, package: &str, op: &str, mode: &str) -> Result<Self, ProfileError> {
        validate_id(id)?;
        AppOpsAdapter::new(package, op, mode, "default")?;
        Ok(Self {
            id: id.to_owned(),
            package: package.to_owned(),
            op: op.to_owned(),
            mode: mode.to_owned(),
        })
    }
}

/// `pm hide` 处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmHidePolicy {
    /// 不执行 `pm hide`。
    DoNotHide,
    /// 尝试执行；不可用时记录 skip。
    TryHide,
}

/// 组件 profile 规则。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentProfileRule {
    pub(crate) id: String,
    pub(crate) user_id: u32,
    pub(crate) component: String,
    pub(crate) package: String,
    pub(crate) hide_policy: PmHidePolicy,
}

impl ComponentProfileRule {
    /// 构造组件禁用规则。
    pub fn new(
        id: &str,
        user_id: u32,
        component: &str,
        hide_policy: PmHidePolicy,
    ) -> Result<Self, ProfileError> {
        validate_id(id)?;
        PmComponentAdapter::new(user_id, component)?;
        let package = package_from_component(component)?;
        Ok(Self {
            id: id.to_owned(),
            user_id,
            component: component.to_owned(),
            package,
            hide_policy,
        })
    }
}

/// ROM 匹配条件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RomMatcher {
    pub(crate) name: String,
    pub(crate) property: String,
}

impl RomMatcher {
    /// MIUI / `HyperOS` 匹配器。
    #[must_use]
    pub fn miui() -> Self {
        Self {
            name: "miui".to_owned(),
            property: "ro.miui.ui.version.name".to_owned(),
        }
    }
}

/// ROM settings 规则。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomSettingsRule {
    pub(crate) namespace: SettingsNamespace,
    pub(crate) key: String,
    pub(crate) value: String,
}

impl RomSettingsRule {
    /// 构造白名单 settings 规则。
    pub fn new(namespace: SettingsNamespace, key: &str, value: &str) -> Result<Self, CommandError> {
        SettingsAdapter::new(namespace, key, value, None)?;
        Ok(Self {
            namespace,
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }
}

/// `shared_prefs` boolean 规则。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedPrefsBoolRule {
    pub(crate) path: PathBuf,
    pub(crate) key: String,
    pub(crate) value: bool,
    pub(crate) backup_dir: PathBuf,
}

impl SharedPrefsBoolRule {
    /// 构造 `shared_prefs` boolean 修改规则。
    pub fn new(
        path: &Path,
        key: &str,
        value: bool,
        backup_dir: &Path,
    ) -> Result<Self, ProfileError> {
        validate_xml_key(key)?;
        Ok(Self {
            path: path.to_path_buf(),
            key: key.to_owned(),
            value,
            backup_dir: backup_dir.to_path_buf(),
        })
    }
}

/// ROM profile 规则。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomProfileRule {
    pub(crate) id: String,
    pub(crate) matcher: RomMatcher,
    pub(super) action: RomProfileAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RomProfileAction {
    Settings(RomSettingsRule),
    SharedPrefsBool(SharedPrefsBoolRule),
}

impl RomProfileRule {
    /// 构造 settings ROM 规则。
    pub fn settings(
        id: &str,
        matcher: RomMatcher,
        rule: RomSettingsRule,
    ) -> Result<Self, ProfileError> {
        validate_id(id)?;
        Ok(Self {
            id: id.to_owned(),
            matcher,
            action: RomProfileAction::Settings(rule),
        })
    }

    /// 构造 `shared_prefs` boolean ROM 规则。
    pub fn shared_prefs_bool(
        id: &str,
        matcher: RomMatcher,
        rule: SharedPrefsBoolRule,
    ) -> Result<Self, ProfileError> {
        validate_id(id)?;
        Ok(Self {
            id: id.to_owned(),
            matcher,
            action: RomProfileAction::SharedPrefsBool(rule),
        })
    }
}

fn validate_id(id: &str) -> Result<(), ProfileError> {
    if id.is_empty()
        || id.len() > 96
        || !id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(ProfileError::invalid_rule(
            "id",
            id,
            "id must be a short ASCII rule token",
        ));
    }
    Ok(())
}

fn validate_xml_key(key: &str) -> Result<(), ProfileError> {
    if key.is_empty() || key.contains(['\0', '\n', '"', '<', '>']) {
        return Err(ProfileError::invalid_rule(
            "xml_key",
            key,
            "xml key must be non-empty and XML attribute safe",
        ));
    }
    Ok(())
}

fn package_from_component(component: &str) -> Result<String, ProfileError> {
    component.split_once('/').map_or_else(
        || {
            Err(ProfileError::invalid_rule(
                "component",
                component,
                "component must include package/name",
            ))
        },
        |(package, _name)| Ok(package.to_owned()),
    )
}
