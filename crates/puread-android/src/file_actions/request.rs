use puread_core::model::{ProfileKind, RiskLevel, RuleId};
use puread_core::restore_ledger::LedgerAction;

use crate::file_actions::metadata::MetadataChange;
use crate::file_actions::target::FileActionTarget;

/// 文件动作类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileActionKind {
    /// 删除目标。
    Delete,
    /// 清空或创建空文件。
    EmptyFile,
    /// 清空或创建空目录。
    EmptyDir,
    /// 将目标权限设置为 000。
    Chmod000,
}

impl FileActionKind {
    pub(super) const fn ledger_action(self) -> LedgerAction {
        match self {
            Self::Delete => LedgerAction::Delete,
            Self::EmptyFile => LedgerAction::EmptyFile,
            Self::EmptyDir => LedgerAction::EmptyDir,
            Self::Chmod000 => LedgerAction::Chmod000,
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::EmptyFile => "empty_file",
            Self::EmptyDir => "empty_dir",
            Self::Chmod000 => "chmod_000",
        }
    }
}

/// 类型化文件动作请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileActionRequest {
    rule_id: RuleId,
    action: FileActionKind,
    target: FileActionTarget,
    profile: ProfileKind,
    risk_level: RiskLevel,
    metadata_changes: Vec<MetadataChange>,
}

impl FileActionRequest {
    /// 创建类型化文件动作请求。
    #[must_use]
    pub const fn new(
        rule_id: RuleId,
        action: FileActionKind,
        target: FileActionTarget,
        profile: ProfileKind,
        risk_level: RiskLevel,
    ) -> Self {
        Self {
            rule_id,
            action,
            target,
            profile,
            risk_level,
            metadata_changes: Vec::new(),
        }
    }

    /// 追加一个元数据封装动作。
    pub fn push_metadata_change(&mut self, change: MetadataChange) {
        self.metadata_changes.push(change);
    }

    pub(super) const fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    pub(super) const fn action(&self) -> FileActionKind {
        self.action
    }

    pub(super) const fn target(&self) -> &FileActionTarget {
        &self.target
    }

    pub(super) const fn profile(&self) -> ProfileKind {
        self.profile
    }

    pub(super) const fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }

    pub(super) fn metadata_changes(&self) -> &[MetadataChange] {
        &self.metadata_changes
    }
}
