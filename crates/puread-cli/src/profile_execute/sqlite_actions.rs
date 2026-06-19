use std::path::Path;

use puread_android::sqlite_actions::{
    SqliteAction, SqliteActionRequest, SqliteActionRunner, SqliteActionSchedule,
    SqliteActionStatus, SqliteActionTarget,
};
use puread_core::model::RuleAction;

use super::ApplyActionReport;
use crate::error::CliError;
use crate::profile_execute::report::file_report;
use crate::rule_plan::PlannedAction;

pub(super) fn execute_action(
    action: &PlannedAction,
    runner: &SqliteActionRunner,
    root: &Path,
) -> ApplyActionReport {
    match request_from_action(action, root) {
        Ok(request) => sqlite_report(action, runner.run_batch(&[request])),
        Err(error) => file_report(action, "failed", Some(error.to_string())),
    }
}

fn sqlite_report(
    action: &PlannedAction,
    report: puread_android::sqlite_actions::BatchReport,
) -> ApplyActionReport {
    let Some(outcome) = report.outcomes.into_iter().next() else {
        return file_report(action, "skipped", None);
    };
    match outcome.status {
        SqliteActionStatus::Applied => file_report(action, "applied", None),
        SqliteActionStatus::Skipped(_reason) => file_report(action, "skipped", None),
        SqliteActionStatus::Failed(failure) => file_report(action, "failed", Some(failure.message)),
        _ => file_report(
            action,
            "failed",
            Some("unknown sqlite action status".to_owned()),
        ),
    }
}

fn request_from_action(
    action: &PlannedAction,
    root: &Path,
) -> Result<SqliteActionRequest, CliError> {
    let target =
        SqliteActionTarget::from_android_path(&action.android_path, &action.host_path, root)
            .map_err(|source| CliError::SqliteAction { source })?;
    Ok(SqliteActionRequest::new(
        action.rule_id.clone(),
        target,
        sqlite_action(&action.action)?,
        sqlite_schedule(action.schedule.as_deref())?,
    ))
}

fn sqlite_action(action: &str) -> Result<SqliteAction, CliError> {
    match RuleAction::parse(action).map_err(CliError::Model)? {
        RuleAction::Delete => Ok(SqliteAction::Delete),
        RuleAction::MinimalSqlite => Ok(SqliteAction::MinimalSqlite),
        RuleAction::DenyWrite => Ok(SqliteAction::DenyWrite),
        other => Err(CliError::UnsupportedProfileAction {
            action: other.as_str(),
        }),
    }
}

fn sqlite_schedule(schedule: Option<&str>) -> Result<SqliteActionSchedule, CliError> {
    match schedule {
        Some("manual") => Ok(SqliteActionSchedule::Manual),
        Some("boot_once") => Ok(SqliteActionSchedule::BootOnce),
        Some("low_frequency") => Ok(SqliteActionSchedule::LowFrequency),
        _ => Err(CliError::InvalidActionTarget {
            path: String::new(),
            reason: "sqlite action requires a supported schedule",
        }),
    }
}
