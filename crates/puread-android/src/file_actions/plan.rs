use puread_core::model::{ProfileKind, RiskLevel, RuleId};

use crate::file_actions::error::FileActionError;
use crate::file_actions::metadata::MetadataChange;
use crate::file_actions::request::{FileActionKind, FileActionRequest};
use crate::file_actions::target::FileActionTarget;

/// 文件动作计划器。
#[derive(Debug, Clone, Copy, Default)]
pub struct FileActionPlanner;

impl FileActionPlanner {
    /// 创建计划器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 生成不可变执行计划。
    pub fn plan(&self, request: &FileActionRequest) -> Result<FileActionPlan, FileActionError> {
        Ok(FileActionPlan {
            rule_id: request.rule_id().clone(),
            action: request.action(),
            target: request.target().clone(),
            profile: request.profile(),
            risk_level: request.risk_level(),
            metadata_changes: request.metadata_changes().to_vec(),
        })
    }
}

/// 已生成的文件动作计划。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileActionPlan {
    rule_id: RuleId,
    action: FileActionKind,
    target: FileActionTarget,
    profile: ProfileKind,
    risk_level: RiskLevel,
    metadata_changes: Vec<MetadataChange>,
}

impl FileActionPlan {
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

    /// 返回规则风险等级。
    #[must_use]
    pub const fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }

    pub(super) fn metadata_changes(&self) -> &[MetadataChange] {
        &self.metadata_changes
    }
}
