#![doc = "恢复账本行为测试。"]

use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use puread_core::restore_ledger::{
    AppendOutcome, LedgerAction, LedgerError, LedgerRecord, OriginalFileType, RestoreAttempt,
    RestoreLedger, RestoreStatus, RestoreStep,
};
use time::macros::datetime;

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Result<Self, LedgerError> {
        let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("puread-restore-ledger-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).map_err(|source| LedgerError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(Self { path })
    }

    fn ledger(&self) -> RestoreLedger {
        RestoreLedger::at(self.path.join("actions.jsonl"))
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.path);
    }
}

fn sample_record(original_path: &str, timestamp: time::OffsetDateTime) -> LedgerRecord {
    LedgerRecord {
        original_path: original_path.to_owned(),
        action: LedgerAction::EmptyFile,
        original_file_type: OriginalFileType::File,
        mode: 0o644,
        uid: 10_000,
        gid: 10_000,
        selinux_context: Some("u:object_r:app_data_file:s0:c123,c456".to_owned()),
        immutable: false,
        timestamp,
        profile: "conservative".to_owned(),
        restore_steps: vec![
            RestoreStep::RestoreContent {
                backup_path: "/data/adb/modules/puread/state/backups/cache-ad.bin".to_owned(),
            },
            RestoreStep::SetMode { mode: 0o644 },
            RestoreStep::SetOwner {
                uid: 10_000,
                gid: 10_000,
            },
        ],
    }
}

#[test]
fn restore_ledger_appends_jsonl_record_when_ledger_missing() -> Result<(), Box<dyn Error>> {
    // Given: a missing JSONL ledger under a temp state directory.
    let dir = TestDir::new()?;
    let ledger = dir.ledger();
    let record = sample_record(
        "/data/data/com.demo/cache/ad.bin",
        datetime!(2026-06-17 0:00 UTC),
    );

    // When: the record is appended through the ledger model.
    let outcome = ledger.append_record(&record)?;

    // Then: one structured JSONL record is readable with all state fields intact.
    let records = ledger.read_records()?;
    println!("restore_ledger_append_record_count={}", records.len());
    assert_eq!(outcome, AppendOutcome::Appended);
    assert_eq!(records.len(), 1);
    let stored = first_record(&records)?;
    assert_eq!(stored.original_path, record.original_path);
    assert_eq!(stored.restore_steps.len(), 3);
    Ok(())
}

#[test]
fn restore_ledger_dedupes_same_path_action_profile_when_appending_again()
-> Result<(), Box<dyn Error>> {
    // Given: a ledger that already contains the action for a path/profile pair.
    let dir = TestDir::new()?;
    let ledger = dir.ledger();
    let record = sample_record(
        "/data/data/com.demo/cache/ad.bin",
        datetime!(2026-06-17 0:00 UTC),
    );

    // When: the same action is appended twice.
    let first = ledger.append_record(&record)?;
    let second = ledger.append_record(&record)?;

    // Then: idempotent append keeps a single JSONL row.
    let records = ledger.read_records()?;
    println!("restore_ledger_dedupe_record_count={}", records.len());
    assert_eq!(first, AppendOutcome::Appended);
    assert_eq!(second, AppendOutcome::AlreadyPresent);
    assert_eq!(records.len(), 1);
    Ok(())
}

#[test]
fn restore_ledger_sorts_deepest_and_newest_records_first_when_planning_restore()
-> Result<(), Box<dyn Error>> {
    // Given: parent and child actions recorded in non-restore order.
    let dir = TestDir::new()?;
    let ledger = dir.ledger();
    ledger.append_record(&sample_record(
        "/data/data/com.demo/cache",
        datetime!(2026-06-17 0:00:01 UTC),
    ))?;
    ledger.append_record(&sample_record(
        "/data/data/com.demo/cache/ad.bin",
        datetime!(2026-06-17 0:00:02 UTC),
    ))?;
    ledger.append_record(&sample_record(
        "/data/data/com.demo/files/ad.bin",
        datetime!(2026-06-17 0:00:03 UTC),
    ))?;

    // When: restore order is requested.
    let records = ledger.records_for_restore()?;

    // Then: deeper paths restore before parents, with deterministic timestamp tie-breaks.
    println!(
        "restore_ledger_sort_order={:?}",
        records
            .iter()
            .map(|record| record.original_path.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        restore_paths(&records),
        [
            "/data/data/com.demo/files/ad.bin",
            "/data/data/com.demo/cache/ad.bin",
            "/data/data/com.demo/cache",
        ]
    );
    Ok(())
}

#[test]
fn restore_ledger_preserves_failed_restore_records_when_attempt_recorded()
-> Result<(), Box<dyn Error>> {
    // Given: a ledger record that failed during a model-level restore attempt.
    let dir = TestDir::new()?;
    let ledger = dir.ledger();
    let record = sample_record(
        "/data/data/com.demo/cache/ad.bin",
        datetime!(2026-06-17 0:00 UTC),
    );
    ledger.append_record(&record)?;

    // When: the restore result is recorded as failed.
    let removed = ledger.apply_restore_attempts(&[RestoreAttempt {
        key: record.key(),
        status: RestoreStatus::Failed,
    }])?;

    // Then: the original record remains for a later retry.
    let records = ledger.read_records()?;
    println!("restore_ledger_failure_preserved_count={}", records.len());
    assert_eq!(removed, 0);
    assert_eq!(records.len(), 1);
    let stored = first_record(&records)?;
    assert_eq!(stored.original_path, record.original_path);
    Ok(())
}

fn restore_paths(records: &[LedgerRecord]) -> Vec<&str> {
    records
        .iter()
        .map(|record| record.original_path.as_str())
        .collect()
}

fn first_record(records: &[LedgerRecord]) -> Result<&LedgerRecord, Box<dyn Error>> {
    records.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "expected one ledger record").into()
    })
}

#[test]
fn restore_ledger_removes_successful_restore_records_when_attempt_recorded()
-> Result<(), Box<dyn Error>> {
    // Given: a ledger record that restored successfully.
    let dir = TestDir::new()?;
    let ledger = dir.ledger();
    let record = sample_record(
        "/data/data/com.demo/cache/ad.bin",
        datetime!(2026-06-17 0:00 UTC),
    );
    ledger.append_record(&record)?;

    // When: the restore result is recorded as succeeded.
    let removed = ledger.apply_restore_attempts(&[RestoreAttempt {
        key: record.key(),
        status: RestoreStatus::Succeeded,
    }])?;

    // Then: the restored record is removed.
    let records = ledger.read_records()?;
    println!("restore_ledger_success_removed_count={}", records.len());
    assert_eq!(removed, 1);
    assert!(records.is_empty());
    Ok(())
}

#[test]
fn restore_ledger_rejects_malformed_jsonl_and_invalid_fields_when_reading_boundary_input()
-> Result<(), Box<dyn Error>> {
    // Given: malformed JSONL at the ledger boundary.
    let dir = TestDir::new()?;
    let ledger = dir.ledger();
    fs::write(ledger.path(), "{not-json}\n")?;

    // When / Then: parsing reports the bad line instead of silently accepting it.
    assert!(matches!(
        ledger.read_records(),
        Err(LedgerError::JsonLine { line: 1, .. })
    ));

    // Given: syntactically valid JSON with an invalid relative original path.
    fs::write(
        ledger.path(),
        r#"{"original_path":"relative","action":"empty_file","original_file_type":"file","mode":420,"uid":10000,"gid":10000,"selinux_context":null,"immutable":false,"timestamp":"2026-06-17T00:00:00Z","profile":"conservative","restore_steps":[{"step":"set_mode","mode":420}]}"#,
    )?;

    // When / Then: field validation still rejects untrusted ledger content.
    assert!(matches!(
        ledger.read_records(),
        Err(LedgerError::InvalidRecord {
            field: "original_path",
            ..
        })
    ));
    Ok(())
}

#[test]
fn restore_ledger_builds_fixed_module_state_path_when_module_id_valid() -> Result<(), Box<dyn Error>>
{
    // Given: a safe module id.
    let ledger = RestoreLedger::for_module("puread")?;

    // When / Then: the model uses the required Magisk module state surface.
    assert_eq!(
        ledger.path(),
        PathBuf::from("/data/adb/modules/puread/state/actions.jsonl").as_path()
    );
    assert!(matches!(
        RestoreLedger::for_module("../escape"),
        Err(LedgerError::InvalidRecord {
            field: "module_id",
            ..
        })
    ));
    Ok(())
}
