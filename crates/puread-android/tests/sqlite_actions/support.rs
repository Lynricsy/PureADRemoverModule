use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use puread_android::sqlite_actions::{
    BatchReport, SqliteAction, SqliteActionRequest, SqliteActionSchedule, SqliteActionTarget,
};
use puread_core::restore_ledger::{LedgerRecord, RestoreLedger};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Result<Self, io::Error> {
        let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("puread-sqlite-actions-{}-{id}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn ledger(&self) -> RestoreLedger {
        RestoreLedger::at(self.path.join("state").join("actions.jsonl"))
    }

    fn db_path(&self, name: &str) -> PathBuf {
        self.host_path(&format!("/data/data/com.example.video/databases/{name}"))
    }

    fn nested_db_path(&self, user: u32, name: &str) -> PathBuf {
        self.host_path(&format!(
            "/data/user/{user}/com.example.video/databases/{name}"
        ))
    }

    fn cache_db_path(&self, name: &str) -> PathBuf {
        self.host_path(&format!("/data/data/com.example.video/cache/{name}"))
    }

    fn adb_db_path(&self, name: &str) -> PathBuf {
        self.host_path(&format!("/data/adb/{name}"))
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.path);
    }
}

fn request(
    id: &'static str,
    dir: &TestDir,
    android_path: &str,
    action: SqliteAction,
    schedule: SqliteActionSchedule,
) -> Result<SqliteActionRequest, Box<dyn Error>> {
    let path = dir.host_path(android_path);
    Ok(SqliteActionRequest::new(
        id,
        SqliteActionTarget::from_android_path(android_path, &path, dir.path())?,
        action,
        schedule,
    ))
}

impl TestDir {
    fn host_path(&self, android_path: &str) -> PathBuf {
        self.path.join(android_path.trim_start_matches('/'))
    }
}

fn assert_all_succeeded(report: &BatchReport) {
    assert_eq!(report.failed, 0);
    assert_eq!(report.succeeded, report.outcomes.len());
}

fn assert_sqlite_integrity_ok(path: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new("sqlite3")
        .arg(path)
        .arg("pragma integrity_check;")
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        output.status.success(),
        "sqlite3 failed stdout={stdout:?} stderr={stderr:?}"
    );
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

fn readonly_bits(metadata: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        if metadata.permissions().readonly() {
            0
        } else {
            u32::MAX
        }
    }
}

fn first_record(records: &[LedgerRecord]) -> Result<&LedgerRecord, Box<dyn Error>> {
    records.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "expected one ledger record").into()
    })
}

fn first_outcome(
    report: &BatchReport,
) -> Result<&puread_android::sqlite_actions::SqliteActionOutcome, Box<dyn Error>> {
    report.outcomes.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "expected one sqlite outcome").into()
    })
}

fn missing_parent() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "missing parent")
}
