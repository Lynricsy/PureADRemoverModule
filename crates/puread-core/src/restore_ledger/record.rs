use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::error::LedgerError;

/// 账本动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerAction {
    /// 删除目标。
    Delete,
    /// 清空文件。
    EmptyFile,
    /// 清空目录。
    EmptyDir,
    /// 收紧权限为 000。
    Chmod000,
    /// 阻止写入。
    DenyWrite,
    /// 写入最小化 `SQLite` 内容。
    MinimalSqlite,
    /// 禁用组件。
    DisableComponent,
    /// 设置 `AppOps`。
    SetAppOp,
    /// 修改 ROM 广告相关设置。
    RomSetting,
}

/// 原始文件类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginalFileType {
    /// 普通文件。
    File,
    /// 目录。
    Directory,
    /// 符号链接。
    Symlink,
    /// 执行动作前目标不存在。
    Missing,
    /// 其他文件系统对象。
    Other,
}

/// 恢复步骤。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum RestoreStep {
    /// 从备份路径恢复内容。
    RestoreContent {
        /// 备份文件路径。
        backup_path: String,
    },
    /// 恢复目录。
    RecreateDirectory,
    /// 恢复空文件。
    RecreateFile,
    /// 移除执行动作创建的占位物。
    RemovePlaceholder,
    /// 恢复权限位。
    SetMode {
        /// 原始 mode。
        mode: u32,
    },
    /// 恢复 uid/gid。
    SetOwner {
        /// 原始 uid。
        uid: u32,
        /// 原始 gid。
        gid: u32,
    },
    /// 恢复 `SELinux` context。
    SetSelinuxContext {
        /// 原始 `SELinux` context。
        context: String,
    },
    /// 恢复 immutable 属性。
    SetImmutable {
        /// 原始 immutable 状态。
        immutable: bool,
    },
}

/// 单条恢复账本记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRecord {
    /// 原始目标路径。
    pub original_path: String,
    /// 已执行动作。
    pub action: LedgerAction,
    /// 动作前文件类型。
    pub original_file_type: OriginalFileType,
    /// 动作前 mode。
    pub mode: u32,
    /// 动作前 uid。
    pub uid: u32,
    /// 动作前 gid。
    pub gid: u32,
    /// 动作前 `SELinux` context。
    pub selinux_context: Option<String>,
    /// 动作前 immutable 状态。
    pub immutable: bool,
    /// 记录时间。
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    /// 触发动作的 profile。
    pub profile: String,
    /// 恢复该记录所需步骤。
    pub restore_steps: Vec<RestoreStep>,
}

impl LedgerRecord {
    /// 返回用于幂等去重和恢复尝试匹配的键。
    pub fn key(&self) -> LedgerKey {
        LedgerKey {
            original_path: self.original_path.clone(),
            action: self.action,
            profile: self.profile.clone(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), LedgerError> {
        validate_absolute_path("original_path", &self.original_path)?;
        validate_profile(&self.profile)?;
        validate_selinux_context(self.selinux_context.as_deref())?;
        for step in &self.restore_steps {
            validate_restore_step(step)?;
        }
        Ok(())
    }
}

/// 账本记录键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LedgerKey {
    /// 原始目标路径。
    pub original_path: String,
    /// 已执行动作。
    pub action: LedgerAction,
    /// 触发动作的 profile。
    pub profile: String,
}

pub(super) const fn action_name(action: LedgerAction) -> &'static str {
    match action {
        LedgerAction::Delete => "delete",
        LedgerAction::EmptyFile => "empty_file",
        LedgerAction::EmptyDir => "empty_dir",
        LedgerAction::Chmod000 => "chmod_000",
        LedgerAction::DenyWrite => "deny_write",
        LedgerAction::MinimalSqlite => "minimal_sqlite",
        LedgerAction::DisableComponent => "disable_component",
        LedgerAction::SetAppOp => "set_appop",
        LedgerAction::RomSetting => "rom_setting",
    }
}

pub(super) fn path_depth(path: &str) -> usize {
    Path::new(path).components().count()
}

pub(super) fn validate_module_id(value: &str) -> Result<(), LedgerError> {
    if value.is_empty() || value.contains(['/', '\\', '\0']) {
        return invalid("module_id", value, "must be a single path segment");
    }
    if value == "." || value == ".." {
        return invalid("module_id", value, "must not escape the module directory");
    }
    Ok(())
}

fn validate_profile(value: &str) -> Result<(), LedgerError> {
    if value.is_empty() || value.contains('\0') {
        return invalid("profile", value, "must be non-empty and contain no NUL");
    }
    Ok(())
}

fn validate_selinux_context(value: Option<&str>) -> Result<(), LedgerError> {
    if let Some(context) = value
        && (context.is_empty() || context.contains('\0'))
    {
        return invalid(
            "selinux_context",
            context,
            "must be non-empty and contain no NUL",
        );
    }
    Ok(())
}

fn validate_restore_step(step: &RestoreStep) -> Result<(), LedgerError> {
    match step {
        RestoreStep::RestoreContent { backup_path } => {
            validate_absolute_path("restore_steps.backup_path", backup_path)
        }
        RestoreStep::SetSelinuxContext { context } => {
            validate_selinux_context(Some(context.as_str()))
        }
        RestoreStep::SetMode { .. }
        | RestoreStep::SetOwner { .. }
        | RestoreStep::SetImmutable { .. }
        | RestoreStep::RecreateDirectory
        | RestoreStep::RecreateFile
        | RestoreStep::RemovePlaceholder => Ok(()),
    }
}

fn validate_absolute_path(field: &'static str, value: &str) -> Result<(), LedgerError> {
    if value.is_empty() || value.contains('\0') {
        return invalid(field, value, "must be non-empty and contain no NUL");
    }
    if !Path::new(value).is_absolute() {
        return invalid(field, value, "must be absolute");
    }
    if Path::new(value)
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return invalid(field, value, "must not contain parent components");
    }
    Ok(())
}

fn invalid(field: &'static str, value: &str, reason: &'static str) -> Result<(), LedgerError> {
    Err(LedgerError::InvalidRecord {
        field,
        value: value.to_owned(),
        reason,
    })
}
