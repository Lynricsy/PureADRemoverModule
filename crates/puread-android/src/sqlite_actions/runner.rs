mod backup_ops;
mod delete;
mod hooks;
mod ledger_state;
mod mutation;
mod write_ops;

use std::path::{Path, PathBuf};

use puread_core::restore_ledger::{
    AppendOutcome, OriginalFileType, RestoreAttempt, RestoreLedger, RestoreStatus,
};

use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::ledger::ledger_record;
use crate::sqlite_actions::metadata::SqliteTargetMetadata;
use crate::sqlite_actions::types::{
    BatchReport, SqliteAction, SqliteActionOutcome, SqliteActionRequest, SqliteActionStatus,
};
use crate::sqlite_actions::validate::{
    ensure_parent_for_mutation, reject_hardlinked_target, require_metadata, validate_request,
    validate_target_for_execution, validate_target_type,
};

#[cfg(test)]
mod backup_tests;
#[cfg(test)]
mod delete_race_tests;
#[cfg(test)]
mod race_tests;
#[cfg(test)]
mod security_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use hooks::{run_after_sqlite_ledger_append_for_tests, run_before_sqlite_backup_for_tests};
use ledger_state::{restore_content_path, validate_existing_backup};

/// `SQLite` 广告数据库动作执行器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteActionRunner {
    ledger: RestoreLedger,
    backup_dir: PathBuf,
}

impl SqliteActionRunner {
    /// 构造执行器，备份目录位于账本同级 `backups/sqlite`。
    #[must_use]
    pub fn new(ledger: RestoreLedger) -> Self {
        let backup_dir = ledger.path().parent().map_or_else(
            || PathBuf::from("backups/sqlite"),
            |path| path.join("backups/sqlite"),
        );
        Self { ledger, backup_dir }
    }

    /// 执行一批 `SQLite` 动作；单条失败不会中断后续请求。
    #[must_use]
    pub fn run_batch(&self, requests: &[SqliteActionRequest]) -> BatchReport {
        let outcomes = requests
            .iter()
            .map(|request| self.run_request(request))
            .collect::<Vec<_>>();
        let failed = outcomes
            .iter()
            .filter(|outcome| matches!(outcome.status, SqliteActionStatus::Failed(_)))
            .count();
        BatchReport {
            succeeded: outcomes.len().saturating_sub(failed),
            failed,
            outcomes,
        }
    }

    fn run_request(&self, request: &SqliteActionRequest) -> SqliteActionOutcome {
        let path = request.target().path().clone();
        let preflight = validate_request(&path, request.schedule()).and_then(|()| {
            validate_target_for_execution(&path, request.target().filesystem_root())
        });
        let metadata = if preflight.is_ok() {
            SqliteTargetMetadata::collect(&path).ok()
        } else {
            None
        };
        let status = match preflight.and_then(|()| self.apply(request, metadata.as_ref())) {
            Ok(true) => SqliteActionStatus::Applied,
            Ok(false) => SqliteActionStatus::Skipped("target missing".to_owned()),
            Err(error) => SqliteActionStatus::Failed(error.into_failure()),
        };
        SqliteActionOutcome {
            rule_id: request.rule_id().to_owned(),
            target: path,
            action: request.action(),
            schedule: request.schedule(),
            metadata,
            status,
        }
    }

    fn apply(
        &self,
        request: &SqliteActionRequest,
        metadata: Option<&SqliteTargetMetadata>,
    ) -> Result<bool, SqliteActionError> {
        let path = request.target().path();
        validate_request(path, request.schedule())?;
        validate_target_for_execution(path, request.target().filesystem_root())?;
        let metadata = require_metadata(path, metadata)?;
        validate_target_type(path, metadata)?;
        reject_hardlinked_target(path, metadata)?;
        if request.action() == SqliteAction::Delete
            && metadata.file_type == OriginalFileType::Missing
        {
            return Ok(false);
        }
        ensure_parent_for_mutation(
            path,
            request.target().filesystem_root(),
            request.action(),
            metadata,
        )?;
        let mut lock = mutation::lock_existing(path, metadata)?;
        backup_ops::guard_locked_file_matches_original(lock.as_ref(), path, metadata)?;
        run_before_sqlite_backup_for_tests(path);
        let probe_record = ledger_record(&self.backup_dir, request, metadata)?;
        let (record, append_outcome, created_backup, reused_existing_record) =
            if let Some(existing_record) =
                ledger_state::existing_ledger_record(&self.ledger, &probe_record)?
            {
                (existing_record, AppendOutcome::AlreadyPresent, None, true)
            } else {
                let created_backup = backup_ops::copy_backup_from_locked_file(
                    request.action(),
                    &mut lock,
                    path,
                    &probe_record,
                )?;
                let append_outcome =
                    self.ledger.append_record(&probe_record).map_err(|source| {
                        SqliteActionError::Ledger {
                            path: path.clone(),
                            source,
                        }
                    })?;
                (probe_record, append_outcome, created_backup, false)
            };
        run_after_sqlite_ledger_append_for_tests(path);
        let backup_path = restore_content_path(&record);
        if reused_existing_record {
            validate_existing_backup(backup_path.as_deref())?;
        }
        if let Err(error) = mutation::guard_and_mutate(
            path,
            request.action(),
            metadata,
            backup_path.as_deref(),
            reused_existing_record,
        ) {
            if !error.preserves_pending_ledger() {
                backup_ops::remove_created_backup(created_backup.as_deref());
            }
            if append_outcome == AppendOutcome::Appended && !error.preserves_pending_ledger() {
                self.remove_pending_record(path, &record)?;
            }
            return Err(error);
        }
        drop(lock);
        Ok(true)
    }

    fn remove_pending_record(
        &self,
        path: &Path,
        record: &puread_core::restore_ledger::LedgerRecord,
    ) -> Result<(), SqliteActionError> {
        self.ledger
            .apply_restore_attempts(&[RestoreAttempt {
                key: record.key(),
                status: RestoreStatus::Succeeded,
            }])
            .map(|_removed| ())
            .map_err(|source| SqliteActionError::Ledger {
                path: path.to_path_buf(),
                source,
            })
    }
}
