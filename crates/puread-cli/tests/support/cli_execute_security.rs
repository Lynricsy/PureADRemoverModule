#![allow(
    clippy::redundant_pub_crate,
    reason = "integration test support helpers are imported from their parent test module"
)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const ANDROID_FS_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/android-fs"
);

pub(crate) fn run_puread<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-cli"))
        .args(args)
        .output()?)
}

pub(crate) fn assert_success(output: &Output) -> Result<(), Box<dyn Error>> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8(output.stderr.clone())?;
    Err(format!("CLI failed: {stderr}").into())
}

pub(crate) fn parse_stdout_json(output: &Output) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&output.stdout)?)
}

pub(crate) fn field<'a>(document: &'a Value, key: &str) -> Result<&'a Value, Box<dyn Error>> {
    document
        .get(key)
        .ok_or_else(|| format!("missing JSON field: {key}").into())
}

pub(crate) fn assert_restore_failed_without_clearing(
    output: &Output,
    ledger: &Path,
    ledger_before: &str,
    expected_error: &str,
) -> Result<(), Box<dyn Error>> {
    assert_success(output)?;
    let document = parse_stdout_json(output)?;
    let failed = field(&document, "failed")?;
    if failed != &serde_json::json!(1) {
        return Err(format!("restore did not report one failed action: {document}").into());
    }
    let actions = field(&document, "actions")?
        .as_array()
        .ok_or("actions must be an array")?;
    let error = actions
        .first()
        .and_then(|action| action.get("error"))
        .and_then(Value::as_str)
        .ok_or("missing failed restore error")?;
    if !error.contains(expected_error) {
        return Err(format!("error did not contain {expected_error:?}: {error}").into());
    }
    if fs::read_to_string(ledger)? != ledger_before {
        return Err("restore cleared or rewrote ledger after failed restore".into());
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct TempFixture {
    pub(crate) root: PathBuf,
}

impl TempFixture {
    pub(crate) fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("puread-cli-{name}-{nanos}"));
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub(crate) fn copy_android_fs(&self) -> Result<PathBuf, Box<dyn Error>> {
        let root = self.root.join("android-fs");
        copy_dir(Path::new(ANDROID_FS_FIXTURE), &root)?;
        Ok(root)
    }

    pub(crate) fn minimal_android_fs(&self) -> Result<PathBuf, Box<dyn Error>> {
        let root = self.root.join("android-fs");
        fs::create_dir_all(root.join("data/adb/modules/PureAD/state/backups"))?;
        fs::create_dir_all(root.join("data/adb/modules/PureAD/run"))?;
        Ok(root)
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _remove_result = fs::remove_dir_all(&self.root);
    }
}

fn copy_dir(source: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(target)?;
    for entry_result in fs::read_dir(source)? {
        let entry = entry_result?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

pub(crate) fn ledger_path(root: &Path) -> PathBuf {
    module_root_path(root).join("state/actions.jsonl")
}

pub(crate) fn backup_path(root: &Path, name: &str) -> PathBuf {
    module_root_path(root).join("state/backups").join(name)
}

pub(crate) fn module_root_path(root: &Path) -> PathBuf {
    root.join("data/adb/modules/PureAD")
}

pub(crate) fn other_module_ledger_is_rejected() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-other-module")?;
    let root = fixture.minimal_android_fs()?;
    let target = root.join("data/data/com.demo/cache/ad.bin");
    fs::create_dir_all(target.parent().ok_or("target path has no parent")?)?;
    fs::write(&target, "keep")?;
    let ledger = root.join("data/adb/modules/OtherModule/state/actions.jsonl");
    write_restore_ledger(
        &ledger,
        "/data/data/com.demo/cache/ad.bin",
        &[remove_placeholder_step()],
    )?;
    let before = fs::read_to_string(&ledger)?;

    let output = run_puread([
        "restore",
        "--execute",
        "--ledger",
        ledger.to_string_lossy().as_ref(),
    ])?;

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("restore ledger path cannot be mapped"),
        "{stderr}"
    );
    assert_eq!(fs::read_to_string(&target)?, "keep");
    assert_eq!(fs::read_to_string(&ledger)?, before);
    Ok(())
}

pub(crate) fn write_restore_ledger(
    ledger: &Path,
    original_path: &str,
    steps: &[&str],
) -> Result<String, Box<dyn Error>> {
    let parent = ledger.parent().ok_or("ledger has no parent")?;
    fs::create_dir_all(parent)?;
    let restore_steps = steps.join(",");
    let record = format!(
        r#"{{"original_path":"{original_path}","action":"empty_file","original_file_type":"file","mode":420,"uid":10000,"gid":10000,"selinux_context":null,"immutable":false,"timestamp":"2026-06-18T00:00:00Z","profile":"conservative","restore_steps":[{restore_steps}]}}"#
    );
    fs::write(ledger, format!("{record}\n"))?;
    Ok(record)
}

pub(crate) fn restore_content_step(path: &Path) -> String {
    format!(
        r#"{{"step":"restore_content","backup_path":"{}"}}"#,
        path.display()
    )
}

pub(crate) const fn recreate_directory_step() -> &'static str {
    r#"{"step":"recreate_directory"}"#
}

pub(crate) const fn recreate_file_step() -> &'static str {
    r#"{"step":"recreate_file"}"#
}

pub(crate) const fn remove_placeholder_step() -> &'static str {
    r#"{"step":"remove_placeholder"}"#
}

pub(crate) const fn set_mode_step(mode: u32) -> &'static str {
    match mode {
        384 => r#"{"step":"set_mode","mode":384}"#,
        _ => r#"{"step":"set_mode","mode":420}"#,
    }
}
