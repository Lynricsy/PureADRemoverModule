use std::path::Path;

use puread_core::restore_ledger::{LedgerRecord, OriginalFileType, RestoreStep};
use time::OffsetDateTime;

use crate::file_actions::plan::FileActionPlan;
use crate::file_actions::request::FileActionKind;
use crate::file_actions::snapshot::TargetSnapshot;

pub(super) fn record_for(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
    backup_path: Option<&Path>,
) -> LedgerRecord {
    LedgerRecord {
        original_path: plan.target().android_path().display().to_string(),
        action: plan.action().ledger_action(),
        original_file_type: snapshot.original_type,
        mode: snapshot.mode,
        uid: snapshot.uid,
        gid: snapshot.gid,
        selinux_context: snapshot.selinux_context.clone(),
        immutable: false,
        timestamp: OffsetDateTime::now_utc(),
        profile: plan.profile().as_str().to_owned(),
        restore_steps: restore_steps(plan.action(), snapshot, backup_path),
    }
}

fn restore_steps(
    action: FileActionKind,
    snapshot: &TargetSnapshot,
    backup_path: Option<&Path>,
) -> Vec<RestoreStep> {
    let mut steps = Vec::new();
    if let Some(path) = backup_path {
        steps.push(RestoreStep::RestoreContent {
            backup_path: path.display().to_string(),
        });
    } else if creates_placeholder(action) {
        steps.push(RestoreStep::RemovePlaceholder);
    }
    if snapshot.original_type != OriginalFileType::Missing {
        steps.push(RestoreStep::SetMode {
            mode: snapshot.mode,
        });
        steps.push(RestoreStep::SetOwner {
            uid: snapshot.uid,
            gid: snapshot.gid,
        });
    }
    if let Some(context) = &snapshot.selinux_context {
        steps.push(RestoreStep::SetSelinuxContext {
            context: context.clone(),
        });
    }
    steps
}

const fn creates_placeholder(action: FileActionKind) -> bool {
    matches!(action, FileActionKind::EmptyFile | FileActionKind::EmptyDir)
}
