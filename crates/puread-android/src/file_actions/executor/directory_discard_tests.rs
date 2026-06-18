use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use puread_core::model::{ProfileKind, RiskLevel, RuleId};
use puread_core::restore_ledger::RestoreStep;

use crate::file_actions::{FileActionKind, FileActionPlanner, FileActionRequest, FileActionTarget};

use super::test_support::temp_root;

#[test]
fn empty_dir_repeated_non_empty_directory_fails_before_discard_move() -> Result<(), Box<dyn Error>>
{
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/repeated-non-empty-dir";
    let host_path = root.join("data/data/com.example.app/cache/repeated-non-empty-dir");
    fs::create_dir_all(&host_path)?;
    fs::write(host_path.join("original.bin"), b"original backup payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("directory-discard-fail-closed")?,
        FileActionKind::EmptyDir,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    executor.execute(&plan)?;
    let backup_path = backup_path_from_ledger(&ledger.read_records()?)?;
    fs::write(
        host_path.join("recreated.bin"),
        b"recreated payload must remain",
    )?;
    let second = executor.execute(&plan);

    assert!(second.is_err());
    assert_eq!(
        fs::read(host_path.join("recreated.bin"))?,
        b"recreated payload must remain"
    );
    assert_eq!(
        fs::read(backup_path.join("original.bin"))?,
        b"original backup payload"
    );
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

fn backup_path_from_ledger(
    records: &[puread_core::restore_ledger::LedgerRecord],
) -> Result<PathBuf, Box<dyn Error>> {
    let record = records.first().ok_or("expected one ledger record")?;
    let path = record
        .restore_steps
        .iter()
        .find_map(restore_content_path)
        .ok_or("expected restore content step")?;
    Ok(path)
}

fn restore_content_path(step: &RestoreStep) -> Option<PathBuf> {
    match step {
        RestoreStep::RestoreContent { backup_path } => Some(Path::new(backup_path).to_path_buf()),
        RestoreStep::RecreateDirectory
        | RestoreStep::RecreateFile
        | RestoreStep::RemovePlaceholder
        | RestoreStep::SetMode { .. }
        | RestoreStep::SetOwner { .. }
        | RestoreStep::SetSelinuxContext { .. }
        | RestoreStep::SetImmutable { .. } => None,
    }
}
