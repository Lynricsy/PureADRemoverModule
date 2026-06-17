use crate::error::ModelError;

const ACTION_FIELD: &str = "action";

/// 规则动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RuleAction {
    /// 删除目标路径。
    Delete,
    /// 清空或创建空文件。
    EmptyFile,
    /// 清空或创建空目录。
    EmptyDir,
    /// 将目标权限收紧为 000。
    Chmod000,
    /// 阻止目标继续写入。
    DenyWrite,
    /// 最小化 `SQLite` 广告库内容。
    MinimalSqlite,
    /// 禁用 Android 组件。
    DisableComponent,
    /// 设置 Android `AppOps` 项。
    SetAppOp,
    /// 应用 ROM 级广告相关配置。
    RomSetting,
}

impl RuleAction {
    /// 解析规则动作。
    pub fn parse(raw: &str) -> Result<Self, ModelError> {
        match raw {
            "delete" => Ok(Self::Delete),
            "empty_file" => Ok(Self::EmptyFile),
            "empty_dir" => Ok(Self::EmptyDir),
            "chmod_000" => Ok(Self::Chmod000),
            "deny_write" => Ok(Self::DenyWrite),
            "minimal_sqlite" => Ok(Self::MinimalSqlite),
            "disable_component" => Ok(Self::DisableComponent),
            "set_appop" => Ok(Self::SetAppOp),
            "rom_setting" => Ok(Self::RomSetting),
            _ => Err(unsupported_action(raw)),
        }
    }

    /// 返回规则文件使用的动作名。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::EmptyFile => "empty_file",
            Self::EmptyDir => "empty_dir",
            Self::Chmod000 => "chmod_000",
            Self::DenyWrite => "deny_write",
            Self::MinimalSqlite => "minimal_sqlite",
            Self::DisableComponent => "disable_component",
            Self::SetAppOp => "set_appop",
            Self::RomSetting => "rom_setting",
        }
    }
}

fn unsupported_action(raw: &str) -> ModelError {
    if raw.is_empty() {
        return ModelError::Empty {
            field: ACTION_FIELD,
        };
    }
    ModelError::UnsupportedValue {
        field: ACTION_FIELD,
        value: raw.to_owned(),
    }
}
