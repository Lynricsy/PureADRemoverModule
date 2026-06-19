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

pub(crate) const ANDROID_FS_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/android-fs"
);
pub(crate) const LEDGER_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/ledger.json"
);

pub(crate) const fn appops_rules() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/appops")
}

pub(crate) const fn component_rules() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/components")
}

pub(crate) const fn rom_rules() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/rom")
}

pub(crate) fn run_puread<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-cli"))
        .args(args)
        .output()?)
}

pub(crate) fn run_puread_with_profile_runner<const N: usize>(
    args: [&str; N],
    runner_log: &Path,
) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-cli"))
        .args(args)
        .arg("--test-profile-runner")
        .arg("--profile-runner-log")
        .arg(runner_log)
        .output()?)
}

pub(crate) fn run_puread_with_failing_profile_ledger<const N: usize>(
    args: [&str; N],
    runner_log: &Path,
) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-cli"))
        .args(args)
        .arg("--test-profile-runner")
        .arg("--profile-runner-log")
        .arg(runner_log)
        .arg("--test-profile-ledger-fail")
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

pub(crate) fn fixture_listing(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut entries = Vec::new();
    collect_listing(root, root, &mut entries)?;
    entries.sort();
    Ok(entries)
}

fn collect_listing(
    root: &Path,
    current: &Path,
    entries: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    for entry_result in fs::read_dir(current)? {
        let entry = entry_result?;
        let path = entry.path();
        let relative = path.strip_prefix(root)?;
        let metadata = fs::symlink_metadata(&path)?;
        let kind = if metadata.is_dir() { "dir" } else { "file" };
        entries.push(format!("{kind}:{}:{}", relative.display(), metadata.len()));
        if metadata.is_dir() {
            collect_listing(root, &path, entries)?;
        }
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct TempFixture {
    root: PathBuf,
    module_root_arg: String,
    lock_path: PathBuf,
}

impl TempFixture {
    pub(crate) fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("puread-cli-{name}-{nanos}"));
        fs::create_dir_all(&root)?;
        let module_root = root.join("module");
        fs::create_dir_all(module_root.join("run"))?;
        let module_root_arg = module_root.to_string_lossy().into_owned();
        let lock_path = module_root.join("run/puread.lock");
        Ok(Self {
            root,
            module_root_arg,
            lock_path,
        })
    }

    pub(crate) fn module_root_str(&self) -> &str {
        &self.module_root_arg
    }

    pub(crate) fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    pub(crate) fn profile_ledger_path(&self) -> PathBuf {
        PathBuf::from(&self.module_root_arg).join("state/profile-actions.jsonl")
    }

    pub(crate) fn actions_ledger_path(&self) -> PathBuf {
        PathBuf::from(&self.module_root_arg).join("state/actions.jsonl")
    }

    pub(crate) fn profile_runner_log(&self) -> PathBuf {
        self.root.join("profile-runner.log")
    }

    pub(crate) fn copy_android_fs(&self) -> Result<PathBuf, Box<dyn Error>> {
        let root = self.root.join("android-fs");
        copy_dir(Path::new(ANDROID_FS_FIXTURE), &root)?;
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
