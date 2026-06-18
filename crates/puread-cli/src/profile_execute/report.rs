use serde::Serialize;

use crate::rule_plan::PlannedAction;

#[derive(Debug, Serialize)]
pub struct ApplyActionReport {
    rule_id: String,
    action: String,
    target_kind: String,
    package: String,
    android_path: String,
    host_path: String,
    status: String,
    record: Option<String>,
    error: Option<String>,
}

#[derive(Debug)]
pub struct ExecutionSummary {
    reports: Vec<ApplyActionReport>,
    applied: usize,
    skipped: usize,
    failed: usize,
}

impl ExecutionSummary {
    pub fn reports(self) -> Vec<ApplyActionReport> {
        self.reports
    }

    pub const fn applied(&self) -> usize {
        self.applied
    }

    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    pub const fn failed(&self) -> usize {
        self.failed
    }

    pub(super) fn from_reports(reports: Vec<ApplyActionReport>) -> Self {
        let applied = reports
            .iter()
            .filter(|item| item.status == "applied")
            .count();
        let skipped = reports
            .iter()
            .filter(|item| item.status == "skipped")
            .count();
        let failed = reports
            .iter()
            .filter(|item| item.status == "failed")
            .count();
        Self {
            reports,
            applied,
            skipped,
            failed,
        }
    }
}

pub(super) fn file_report(
    action: &PlannedAction,
    status: &str,
    error: Option<String>,
) -> ApplyActionReport {
    action_report(action, status, None, error)
}

pub(super) fn android_report(
    action: &PlannedAction,
    status: &str,
    record: Option<String>,
    error: Option<String>,
) -> ApplyActionReport {
    action_report(action, status, record, error)
}

fn action_report(
    action: &PlannedAction,
    status: &str,
    record: Option<String>,
    error: Option<String>,
) -> ApplyActionReport {
    ApplyActionReport {
        rule_id: action.rule_id.clone(),
        action: action.action.clone(),
        target_kind: action.target_kind.clone(),
        package: action.package.clone(),
        android_path: action.android_path.clone(),
        host_path: action.host_path.clone(),
        status: status.to_owned(),
        record,
        error,
    }
}
