use serde::{Deserialize, Serialize};

use crate::command_runner::SettingsNamespace;

/// Android profile 操作状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileOperationStatus {
    /// 已执行变更。
    Applied,
    /// 因能力或 ROM 不匹配跳过。
    Skipped,
}

/// Android profile 操作结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileOperation {
    /// 状态。
    pub status: ProfileOperationStatus,
    /// JSON 格式恢复记录。
    pub record: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ProfileRecord {
    AppSkipped(AppSkippedRecord),
    AppOp(AppOpRecord),
    Component(ComponentRecord),
    RomSetting(RomSettingRecord),
    SharedPrefsBool(SharedPrefsBoolRecord),
    RomSkipped(RomSkippedRecord),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct AppSkippedRecord {
    pub rule_id: String,
    pub package: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct AppOpRecord {
    pub rule_id: String,
    pub package: String,
    pub op: String,
    pub applied_mode: String,
    pub original_mode: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PackageEnabledState {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PackageHiddenState {
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum HideApplyStatus {
    NotRequested,
    Applied,
    SkippedUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ComponentRecord {
    pub rule_id: String,
    pub user_id: u32,
    pub package: String,
    pub component: String,
    pub original_enabled: PackageEnabledState,
    pub original_hidden: PackageHiddenState,
    pub hide_status: HideApplyStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RomSettingRecord {
    pub rule_id: String,
    pub matcher: String,
    pub namespace: SettingsNamespace,
    pub key: String,
    pub applied_value: String,
    pub original_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SharedPrefsBoolRecord {
    pub rule_id: String,
    pub matcher: String,
    pub path: String,
    pub key: String,
    pub applied_value: bool,
    pub original_value: bool,
    pub original_sha256: String,
    pub backup_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RomSkippedRecord {
    pub rule_id: String,
    pub matcher: String,
    pub reason: String,
}
