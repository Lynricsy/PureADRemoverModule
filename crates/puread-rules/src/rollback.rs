use crate::RuleCategory;
use crate::error::RuleParseError;

/// 恢复策略枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RollbackStrategy {
    /// 恢复原文件或目录状态。
    RestoreOriginal,
    /// 重新启用被禁用组件。
    ReenableComponent,
    /// 恢复 `AppOps` 原值。
    RestoreAppOp,
    /// 恢复 ROM 设置原值。
    RestoreRomValue,
}

impl RollbackStrategy {
    pub(super) fn parse(raw: &str) -> Result<Self, RuleParseError> {
        match raw {
            "restore_original" => Ok(Self::RestoreOriginal),
            "reenable_component" => Ok(Self::ReenableComponent),
            "restore_appop" => Ok(Self::RestoreAppOp),
            "restore_rom_value" => Ok(Self::RestoreRomValue),
            _ if raw.trim().is_empty() => Err(RuleParseError::EmptyMetadata {
                field: "rollback_strategy",
            }),
            _ => Err(RuleParseError::InvalidTarget {
                category: RuleCategory::FilePath,
                reason: "unsupported rollback_strategy",
            }),
        }
    }

    /// 返回规则文件中使用的恢复策略名。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RestoreOriginal => "restore_original",
            Self::ReenableComponent => "reenable_component",
            Self::RestoreAppOp => "restore_appop",
            Self::RestoreRomValue => "restore_rom_value",
        }
    }
}
