use std::path::{Path, PathBuf};

use puread_core::restore_ledger::{
    AppendOutcome, OriginalFileType, RestoreAttempt, RestoreLedger, RestoreStatus, RestoreStep,
};

use crate::file_actions::backup::backup_original;
use crate::file_actions::error::FileActionError;
use crate::file_actions::ledger::record_for;
use crate::file_actions::mutate::{apply_metadata_changes, apply_primary_action};
use crate::file_actions::outcome::FileActionOutcome;
use crate::file_actions::plan::FileActionPlan;
use crate::file_actions::request::FileActionKind;
use crate::file_actions::snapshot::{TargetSnapshot, identity_changed};
use crate::secure_fs::ensure_parent_beneath_root_no_symlink;

#[cfg(test)]
mod directory_discard_tests;
#[cfg(test)]
mod idempotency_tests;
#[cfg(test)]
mod race_tests;
#[cfg(test)]
mod security_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

/// 文件动作执行器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileActionExecutor {
    ledger: RestoreLedger,
    backup_dir: PathBuf,
}

impl FileActionExecutor {
    /// 创建执行器。
    #[must_use]
    pub const fn new(ledger: RestoreLedger, backup_dir: PathBuf) -> Self {
        Self { ledger, backup_dir }
    }

    /// 只检查计划可用性，不写账本、不修改目标。
    pub fn dry_run(&self, _plan: &FileActionPlan) -> Result<FileActionOutcome, FileActionError> {
        validate_backup_dir(&self.backup_dir)?;
        Ok(FileActionOutcome::planned())
    }

    /// 真实执行文件动作。
    pub fn execute(&self, plan: &FileActionPlan) -> Result<FileActionOutcome, FileActionError> {
        validate_backup_dir(&self.backup_dir)?;
        guard_target_parent_inside_root(plan)?;
        let snapshot = TargetSnapshot::collect(plan.target())?;
        if should_skip_missing(plan.action(), snapshot.original_type) {
            return Ok(FileActionOutcome::skipped());
        }
        reject_hardlinked_regular_file(plan, &snapshot)?;
        let probe_record = record_for(plan, &snapshot, None);
        let (record, append_outcome, backup_path, reused_existing_record) =
            if let Some(existing_record) = self.existing_ledger_record(&probe_record)? {
                let backup_path = restore_content_path(&existing_record)?;
                (
                    existing_record,
                    AppendOutcome::AlreadyPresent,
                    backup_path,
                    true,
                )
            } else {
                let backup_path = backup_original(&self.backup_dir, plan, &snapshot)?;
                let record = record_for(plan, &snapshot, backup_path.as_deref());
                let append_outcome = self.ledger.append_record(&record)?;
                (record, append_outcome, backup_path, false)
            };
        if reused_existing_record {
            validate_existing_backup(backup_path.as_deref())?;
        }
        if let Err(error) = guard_and_apply_primary_action(
            plan,
            &snapshot,
            backup_path.as_deref(),
            reused_existing_record,
        ) {
            if append_outcome == AppendOutcome::Appended && !error.preserves_pending_ledger() {
                self.remove_pending_record(&record)?;
            }
            return Err(error);
        }
        let metadata_operations = apply_metadata_changes(plan, &snapshot)?;
        Ok(FileActionOutcome::applied(metadata_operations))
    }

    fn existing_ledger_record(
        &self,
        record: &puread_core::restore_ledger::LedgerRecord,
    ) -> Result<Option<puread_core::restore_ledger::LedgerRecord>, FileActionError> {
        let key = record.key();
        Ok(self
            .ledger
            .read_records()?
            .iter()
            .find(|stored| stored.key() == key)
            .cloned())
    }

    fn remove_pending_record(
        &self,
        record: &puread_core::restore_ledger::LedgerRecord,
    ) -> Result<(), FileActionError> {
        self.ledger.apply_restore_attempts(&[RestoreAttempt {
            key: record.key(),
            status: RestoreStatus::Succeeded,
        }])?;
        Ok(())
    }
}

fn guard_and_apply_primary_action(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
    backup_path: Option<&Path>,
    reused_existing_record: bool,
) -> Result<(), FileActionError> {
    guard_target_unchanged(plan, snapshot)?;
    apply_primary_action(plan, snapshot, backup_path, reused_existing_record)
}

fn guard_target_unchanged(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
) -> Result<(), FileActionError> {
    guard_target_parent_inside_root(plan)?;
    let current = TargetSnapshot::collect(plan.target())?;
    if current.original_type != snapshot.original_type {
        return Err(FileActionError::rejected_target(
            plan.target().android_path(),
            "target changed before mutation",
        ));
    }
    if identity_changed(snapshot.identity, current.identity) {
        return Err(FileActionError::rejected_target(
            plan.target().android_path(),
            "target identity changed before mutation",
        ));
    }
    Ok(())
}

fn guard_target_parent_inside_root(plan: &FileActionPlan) -> Result<(), FileActionError> {
    ensure_parent_beneath_root_no_symlink(
        plan.target().host_path(),
        plan.target().filesystem_root(),
    )
    .map_err(|source| FileActionError::io(plan.target().host_path(), source))
}

fn should_skip_missing(action: FileActionKind, original_type: OriginalFileType) -> bool {
    original_type == OriginalFileType::Missing
        && matches!(action, FileActionKind::Delete | FileActionKind::Chmod000)
}

fn reject_hardlinked_regular_file(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
) -> Result<(), FileActionError> {
    if snapshot.original_type == OriginalFileType::File && snapshot.nlink > 1 {
        return Err(FileActionError::rejected_target(
            plan.target().android_path(),
            "hardlinked file target rejected",
        ));
    }
    Ok(())
}

fn validate_backup_dir(path: &std::path::Path) -> Result<(), FileActionError> {
    if path.as_os_str().is_empty() {
        return Err(FileActionError::rejected_target(
            path,
            "backup dir must not be empty",
        ));
    }
    Ok(())
}

fn validate_existing_backup(backup_path: Option<&Path>) -> Result<(), FileActionError> {
    let Some(path) = backup_path else {
        return Ok(());
    };
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_symlink() => return Ok(()),
        Ok(_metadata) => {
            return Err(FileActionError::rejected_target(
                path,
                "stored backup path is symlink",
            ));
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(FileActionError::io(path, source)),
    }
    Err(FileActionError::rejected_target(
        path,
        "stored backup missing before repeated mutation",
    ))
}

fn restore_content_path(
    record: &puread_core::restore_ledger::LedgerRecord,
) -> Result<Option<PathBuf>, FileActionError> {
    record
        .restore_steps
        .iter()
        .find_map(|step| match step {
            RestoreStep::RestoreContent { backup_path } => Some(PathBuf::from(backup_path)),
            RestoreStep::RecreateDirectory
            | RestoreStep::RecreateFile
            | RestoreStep::RemovePlaceholder
            | RestoreStep::SetMode { .. }
            | RestoreStep::SetOwner { .. }
            | RestoreStep::SetSelinuxContext { .. }
            | RestoreStep::SetImmutable { .. } => None,
        })
        .map_or(Ok(None), |path| {
            if path.as_os_str().is_empty() {
                return Err(FileActionError::rejected_target(
                    path,
                    "stored backup path is empty",
                ));
            }
            Ok(Some(path))
        })
}
