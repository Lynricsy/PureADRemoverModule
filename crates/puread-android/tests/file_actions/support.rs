use std::error::Error as StdError;
use std::fs as test_fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use puread_android::file_actions::{
    FileActionKind as SupportFileActionKind, FileActionPlan as SupportFileActionPlan,
    FileActionPlanner as SupportFileActionPlanner, FileActionRequest as SupportFileActionRequest,
    FileActionTarget,
};
use puread_core::model::{ProfileKind, RiskLevel as SupportRiskLevel, RuleId};
use puread_core::restore_ledger::{LedgerRecord, RestoreLedger, RestoreStep as SupportRestoreStep};

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    fn new() -> Result<Self, Box<dyn StdError>> {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("puread-file-actions-{}-{id}", std::process::id()));
        if path.exists() {
            test_fs::remove_dir_all(&path)?;
        }
        test_fs::create_dir_all(path.join("state/backups"))?;
        Ok(Self { path })
    }

    fn host_path(&self, android_path: &str) -> PathBuf {
        self.path.join(android_path.trim_start_matches('/'))
    }

    fn write(&self, android_path: &str, content: &[u8]) -> Result<(), Box<dyn StdError>> {
        let host_path = self.host_path(android_path);
        if let Some(parent) = host_path.parent() {
            test_fs::create_dir_all(parent)?;
        }
        test_fs::write(host_path, content)?;
        Ok(())
    }

    fn mkdir(&self, android_path: &str) -> Result<(), Box<dyn StdError>> {
        test_fs::create_dir_all(self.host_path(android_path))?;
        Ok(())
    }

    fn target(&self, android_path: &str) -> Result<FileActionTarget, Box<dyn StdError>> {
        Ok(FileActionTarget::new(
            android_path,
            self.host_path(android_path),
            self.path.as_path(),
        )?)
    }

    fn ledger(&self) -> RestoreLedger {
        RestoreLedger::at(self.path.join("state/actions.jsonl"))
    }

    fn backup_dir(&self) -> PathBuf {
        self.path.join("state/backups")
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ignored = test_fs::remove_dir_all(&self.path);
    }
}

fn plan_for(
    root: &TestRoot,
    android_path: &str,
    action: SupportFileActionKind,
    risk: SupportRiskLevel,
) -> Result<SupportFileActionPlan, Box<dyn StdError>> {
    let request = request_for(root, android_path, action, risk)?;
    Ok(SupportFileActionPlanner::new().plan(&request)?)
}

fn request_for(
    root: &TestRoot,
    android_path: &str,
    action: SupportFileActionKind,
    risk: SupportRiskLevel,
) -> Result<SupportFileActionRequest, Box<dyn StdError>> {
    Ok(SupportFileActionRequest::new(
        RuleId::parse("file-actions-test")?,
        action,
        root.target(android_path)?,
        ProfileKind::Conservative,
        risk,
    ))
}

fn first_record(records: &[LedgerRecord]) -> Result<&LedgerRecord, Box<dyn StdError>> {
    records.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "expected one ledger record").into()
    })
}

fn restore_backup_path(record: &LedgerRecord) -> Result<PathBuf, Box<dyn StdError>> {
    record
        .restore_steps
        .iter()
        .find_map(|step| match step {
            SupportRestoreStep::RestoreContent { backup_path } => Some(PathBuf::from(backup_path)),
            _other => None,
        })
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "expected restore content step").into()
        })
}
