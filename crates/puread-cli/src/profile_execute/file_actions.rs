use std::path::Path;

use puread_android::file_actions::{
    FileActionExecutor, FileActionKind, FileActionPlanner, FileActionRequest, FileActionStatus,
    FileActionTarget,
};
use puread_core::model::{ProfileKind, RiskLevel, RuleAction, RuleId};

use super::ApplyActionReport;
use crate::error::CliError;
use crate::profile_execute::report::file_report;
use crate::rule_plan::PlannedAction;

pub(super) fn execute_action(
    action: &PlannedAction,
    executor: &FileActionExecutor,
    root: &Path,
) -> ApplyActionReport {
    match apply_action(action, executor, root) {
        Ok(status) => file_report(action, status, None),
        Err(error) => file_report(action, "failed", Some(error.to_string())),
    }
}

fn apply_action(
    action: &PlannedAction,
    executor: &FileActionExecutor,
    root: &Path,
) -> Result<&'static str, CliError> {
    if action.target_kind != "path" {
        return Err(CliError::UnsupportedProfileAction {
            action: RuleAction::parse(&action.action)
                .map_err(CliError::Model)?
                .as_str(),
        });
    }
    let request = FileActionRequest::new(
        RuleId::parse(&action.rule_id)?,
        file_action_kind(&action.action)?,
        FileActionTarget::new(&action.android_path, &action.host_path, root)?,
        ProfileKind::parse(&action.profile)?,
        RiskLevel::parse(&action.risk_level)?,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let outcome = executor.execute(&plan)?;
    match outcome.status() {
        FileActionStatus::Applied => Ok("applied"),
        FileActionStatus::Skipped => Ok("skipped"),
        FileActionStatus::Planned => Ok("planned"),
        _ => Err(CliError::InvalidActionTarget {
            path: action.host_path.clone(),
            reason: "unknown file action status",
        }),
    }
}

fn file_action_kind(action: &str) -> Result<FileActionKind, CliError> {
    match RuleAction::parse(action).map_err(CliError::Model)? {
        RuleAction::Delete => Ok(FileActionKind::Delete),
        RuleAction::EmptyFile => Ok(FileActionKind::EmptyFile),
        RuleAction::EmptyDir => Ok(FileActionKind::EmptyDir),
        RuleAction::Chmod000 => Ok(FileActionKind::Chmod000),
        other => Err(CliError::UnsupportedProfileAction {
            action: other.as_str(),
        }),
    }
}
