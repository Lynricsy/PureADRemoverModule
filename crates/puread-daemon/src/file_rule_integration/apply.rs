use std::path::PathBuf;

use puread_android::file_actions::{
    FileActionExecutor, FileActionKind, FileActionOutcome, FileActionPlanner, FileActionRequest,
    FileActionTarget,
};
use puread_core::model::RuleAction;
use puread_core::restore_ledger::RestoreLedger;

use crate::DaemonError;
use crate::file_rule_integration::planner::ApplyFileAction;

/// daemon 文件规则真实执行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRuleApplyOutcome {
    rule_id: String,
    outcome: FileActionOutcome,
}

impl FileRuleApplyOutcome {
    /// 返回规则 ID。
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }

    /// 返回底层文件动作结果。
    pub const fn outcome(&self) -> &FileActionOutcome {
        &self.outcome
    }

    /// 返回本次结果是否修改了文件系统。
    pub const fn will_mutate(&self) -> bool {
        self.outcome.will_mutate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileRuleApplyExecutor {
    ledger_path: PathBuf,
    backup_dir: PathBuf,
    filesystem_root: PathBuf,
}

impl FileRuleApplyExecutor {
    pub(super) fn new(ledger_path: PathBuf, filesystem_root: PathBuf) -> Self {
        let backup_dir = ledger_path.parent().map_or_else(
            || PathBuf::from("backups/file-rules"),
            |parent| parent.join("backups/file-rules"),
        );
        Self {
            ledger_path,
            backup_dir,
            filesystem_root,
        }
    }

    pub(super) fn execute(
        &self,
        actions: &[ApplyFileAction],
    ) -> Result<Vec<FileRuleApplyOutcome>, DaemonError> {
        let ledger = RestoreLedger::at(self.ledger_path.clone());
        let executor = FileActionExecutor::new(ledger, self.backup_dir.clone());
        actions
            .iter()
            .map(|action| self.execute_one(action, &executor))
            .collect()
    }

    fn execute_one(
        &self,
        action: &ApplyFileAction,
        executor: &FileActionExecutor,
    ) -> Result<FileRuleApplyOutcome, DaemonError> {
        let target = FileActionTarget::new(
            action.android_path(),
            action.host_path(),
            self.filesystem_root.as_path(),
        )
        .map_err(|source| DaemonError::FileAction { source })?;
        let request = FileActionRequest::new(
            action.rule_id().clone(),
            file_action_kind(action.action())?,
            target,
            action.profile(),
            action.risk_level(),
        );
        let plan = FileActionPlanner::new()
            .plan(&request)
            .map_err(|source| DaemonError::FileAction { source })?;
        let outcome = executor
            .execute(&plan)
            .map_err(|source| DaemonError::FileAction { source })?;
        Ok(FileRuleApplyOutcome {
            rule_id: action.rule_id().as_str().to_owned(),
            outcome,
        })
    }
}

const fn file_action_kind(action: RuleAction) -> Result<FileActionKind, DaemonError> {
    match action {
        RuleAction::Delete => Ok(FileActionKind::Delete),
        RuleAction::EmptyFile => Ok(FileActionKind::EmptyFile),
        RuleAction::EmptyDir => Ok(FileActionKind::EmptyDir),
        RuleAction::Chmod000 => Ok(FileActionKind::Chmod000),
        _ => Err(DaemonError::UnsupportedFileAction { action }),
    }
}
