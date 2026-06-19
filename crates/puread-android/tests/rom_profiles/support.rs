#![allow(
    clippy::redundant_pub_crate,
    reason = "integration test support helpers are imported from sibling and parent test modules"
)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use puread_android::command_runner::{
    AndroidCommandRunner, CommandInvocation, CommandOutput, CommandRunnerError,
};
use puread_android::profiles::{ProfileError, ProfileLedgerSink};

#[derive(Debug, Default)]
pub(crate) struct ScriptedRunner {
    outputs: RefCell<VecDeque<CommandOutput>>,
    calls: RefCell<Vec<CommandInvocation>>,
}

impl ScriptedRunner {
    pub(crate) fn with_outputs(outputs: Vec<CommandOutput>) -> Self {
        Self {
            outputs: RefCell::new(VecDeque::from(outputs)),
            calls: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn call_lines(&self) -> Vec<String> {
        self.calls
            .borrow()
            .iter()
            .map(CommandInvocation::argv)
            .map(|argv| argv.join(" "))
            .collect()
    }
}

impl AndroidCommandRunner for ScriptedRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
        self.calls.borrow_mut().push(invocation.clone());
        self.outputs
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| CommandRunnerError::Unavailable {
                detail: "unscripted fake command".to_owned(),
            })
    }
}

#[derive(Debug, Default)]
pub(crate) struct MemoryLedger {
    pub(crate) records: RefCell<Vec<String>>,
    fail: bool,
}

impl ProfileLedgerSink for MemoryLedger {
    fn append(&self, record: String) -> Result<(), ProfileError> {
        if self.fail {
            return Err(ProfileError::Runner {
                detail: "ledger sink failed".to_owned(),
            });
        }
        self.records.borrow_mut().push(record);
        Ok(())
    }
}

impl MemoryLedger {
    pub(crate) const fn failing() -> Self {
        Self {
            records: RefCell::new(Vec::new()),
            fail: true,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TestTempDir {
    path: PathBuf,
}

impl Deref for TestTempDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path.as_path()
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn unique_temp_dir() -> TestTempDir {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let path =
        std::env::temp_dir().join(format!("puread-rom-profile-{}-{nanos}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    let _ = fs::create_dir_all(&path);
    TestTempDir { path }
}

pub(crate) fn write_prefs_fixture(root: &Path) -> Result<PathBuf, io::Error> {
    let prefs = root.join("data/user/0/com.miui.weather2/shared_prefs/prefs.xml");
    fs::create_dir_all(prefs.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "fixture path has no parent")
    })?)?;
    fs::write(
        &prefs,
        r#"<map><boolean name="key_content_promotion" value="true" /></map>"#,
    )?;
    Ok(prefs)
}

pub(crate) fn extract_backup_path(record: &str) -> Result<&str, io::Error> {
    let marker = "\"backup_path\":\"";
    let start = record
        .find(marker)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing backup_path"))?
        + marker.len();
    let tail = &record[start..];
    let end = tail
        .find('"')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unterminated backup_path"))?;
    Ok(&tail[..end])
}
