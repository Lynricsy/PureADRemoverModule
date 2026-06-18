use std::path::{Path, PathBuf};

use puread_core::restore_ledger::{LedgerAction, LedgerRecord, OriginalFileType, RestoreStep};
use time::OffsetDateTime;

use crate::sqlite_actions::backup::backup_path_for;
use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::metadata::SqliteTargetMetadata;
use crate::sqlite_actions::types::{SqliteAction, SqliteActionRequest};

pub(super) fn ledger_record(
    backup_dir: &Path,
    request: &SqliteActionRequest,
    metadata: &SqliteTargetMetadata,
) -> Result<LedgerRecord, SqliteActionError> {
    let path = request.target().path();
    Ok(LedgerRecord {
        original_path: path.display().to_string(),
        action: ledger_action(request.action()),
        original_file_type: metadata.file_type,
        mode: metadata.mode,
        uid: metadata.uid,
        gid: metadata.gid,
        selinux_context: None,
        immutable: false,
        timestamp: OffsetDateTime::now_utc(),
        profile: request.schedule().ledger_profile().to_owned(),
        restore_steps: restore_steps(backup_dir, request, metadata)?,
    })
}

fn restore_steps(
    backup_dir: &Path,
    request: &SqliteActionRequest,
    metadata: &SqliteTargetMetadata,
) -> Result<Vec<RestoreStep>, SqliteActionError> {
    if metadata.file_type == OriginalFileType::Missing {
        return Ok(vec![RestoreStep::RemovePlaceholder]);
    }
    let backup_path: PathBuf =
        backup_path_for(backup_dir, request.target().path(), request.rule_id())?;
    Ok(vec![
        RestoreStep::RestoreContent {
            backup_path: backup_path.display().to_string(),
        },
        RestoreStep::SetMode {
            mode: metadata.mode,
        },
        RestoreStep::SetOwner {
            uid: metadata.uid,
            gid: metadata.gid,
        },
    ])
}

const fn ledger_action(action: SqliteAction) -> LedgerAction {
    match action {
        SqliteAction::Delete => LedgerAction::Delete,
        SqliteAction::MinimalSqlite => LedgerAction::MinimalSqlite,
        SqliteAction::DenyWrite => LedgerAction::DenyWrite,
    }
}
