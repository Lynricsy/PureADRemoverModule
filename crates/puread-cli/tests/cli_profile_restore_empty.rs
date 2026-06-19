#![doc = "CLI profile-restore 空 ledger dry-run 回归测试。"]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn run_puread<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-cli"))
        .args(args)
        .env("PUREAD_TEST_PROFILE_RUNNER", "1")
        .output()?)
}

fn assert_success(output: &Output) -> Result<(), Box<dyn Error>> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8(output.stderr.clone())?;
    Err(format!("CLI failed: {stderr}").into())
}

fn parse_stdout_json(output: &Output) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn field<'a>(document: &'a Value, key: &str) -> Result<&'a Value, Box<dyn Error>> {
    document
        .get(key)
        .ok_or_else(|| format!("missing JSON field: {key}").into())
}

#[derive(Debug)]
struct TempFixture {
    root: PathBuf,
    module_root: PathBuf,
    module_root_arg: String,
}

impl TempFixture {
    fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("puread-cli-{name}-{nanos}"));
        let module_root = root.join("module");
        fs::create_dir_all(module_root.join("state"))?;
        fs::create_dir_all(module_root.join("run"))?;
        let module_root_arg = module_root.to_string_lossy().into_owned();
        Ok(Self {
            root,
            module_root,
            module_root_arg,
        })
    }

    fn module_root(&self) -> &Path {
        self.module_root.as_path()
    }

    const fn module_root_str(&self) -> &str {
        self.module_root_arg.as_str()
    }

    fn runner_log(&self) -> PathBuf {
        self.root.join("profile-runner.log")
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn cli_profile_restore_dry_run_reports_empty_module_root_without_mutation()
-> Result<(), Box<dyn Error>> {
    // Given: a first-use module root has no profile state or lock directory yet.
    let fixture = TempFixture::new("empty-module-dry-run")?;
    fs::remove_dir_all(fixture.module_root().join("state"))?;
    fs::remove_dir_all(fixture.module_root().join("run"))?;

    // When: profile-restore is run in dry-run mode.
    let output = run_puread([
        "profile-restore",
        "--dry-run",
        "--module-root",
        fixture.module_root_str(),
    ])?;

    // Then: the empty ledger is reported as a non-mutating no-op.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "command")?, "profile_restore");
    assert_eq!(field(&document, "mode")?, "dry_run");
    assert_eq!(field(&document, "record_count")?, 0);
    assert_eq!(field(&document, "pending_restore_count")?, 0);
    assert_eq!(field(&document, "will_mutate")?, false);
    assert!(!fixture.module_root().join("state").exists());
    assert!(!fixture.module_root().join("run").exists());
    assert!(!fixture.runner_log().exists());
    Ok(())
}
