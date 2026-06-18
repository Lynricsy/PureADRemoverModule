#![allow(
    clippy::redundant_pub_crate,
    reason = "integration test support helpers are imported from their parent test module"
)]

use std::error::Error;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt as _;
use serde_json::Value;

pub(crate) const ANDROID_FS_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/android-fs"
);

pub(crate) const fn appops_rules() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/appops")
}

pub(crate) fn run_puread<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-cli"))
        .args(args)
        .env("PUREAD_TEST_PROFILE_RUNNER", "1")
        .output()?)
}

pub(crate) fn run_puread_with_profile_test<const N: usize>(
    args: [&str; N],
    _fixture: &TempFixture,
) -> Result<Output, Box<dyn Error>> {
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

pub(crate) fn assert_failed_without_runner_mutation(
    output: &Output,
    fixture: &TempFixture,
) -> Result<(), Box<dyn Error>> {
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr.clone())?;
    assert!(stderr.contains("symlink"), "{stderr}");
    assert!(!fixture.runner_log().exists());
    Ok(())
}

pub(crate) fn assert_lock_failed_without_runner_mutation(
    output: &Output,
    fixture: &TempFixture,
) -> Result<(), Box<dyn Error>> {
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr.clone())?;
    assert!(stderr.contains("global lock is already held"), "{stderr}");
    assert!(!fixture.runner_log().exists());
    Ok(())
}

pub(crate) fn profile_records(fixture: &TempFixture) -> Result<String, Box<dyn Error>> {
    let backup = fixture.shared_prefs_backup_path();
    fs::write(
        &backup,
        "<map><boolean name=\"key_content_promotion\" value=\"true\" /></map>",
    )?;
    Ok(format!(
        "{}{}{}{}",
        appop_record(),
        component_record(),
        rom_setting_record(),
        shared_prefs_record(fixture, &backup)
    ))
}

pub(crate) const fn appop_record() -> &'static str {
    "{\"kind\":\"app_op\",\"rule_id\":\"luna-music-ad-related-appops\",\"package\":\"com.luna.music\",\"op\":\"MONITOR_LOCATION\",\"applied_mode\":\"ignore\",\"original_mode\":\"default\"}\n"
}

const fn component_record() -> &'static str {
    "{\"kind\":\"component\",\"rule_id\":\"mi-market-reverse-ad-page\",\"user_id\":0,\"package\":\"com.xiaomi.market\",\"component\":\"com.xiaomi.market/com.xiaomi.market.reverse_ad.page.WebReverseAdActivity\",\"original_enabled\":\"enabled\",\"original_hidden\":\"visible\",\"hide_status\":\"not_requested\"}\n"
}

const fn rom_setting_record() -> &'static str {
    "{\"kind\":\"rom_setting\",\"rule_id\":\"miui-personalized-ad-enabled\",\"matcher\":\"miui\",\"namespace\":\"global\",\"key\":\"miui_personalized_ad_enabled\",\"applied_value\":\"0\",\"original_value\":\"1\"}\n"
}

fn shared_prefs_record(fixture: &TempFixture, backup: &Path) -> String {
    format!(
        "{{\"kind\":\"shared_prefs_bool\",\"rule_id\":\"miui-weather-content-promotion\",\"matcher\":\"miui\",\"path\":\"{}\",\"key\":\"key_content_promotion\",\"applied_value\":false,\"original_value\":true,\"original_sha256\":\"fixture\",\"backup_path\":\"{}\"}}\n",
        fixture.shared_prefs_path().display(),
        backup.display()
    )
}

#[cfg(unix)]
pub(crate) fn state_symlink_is_rejected_before_mutation() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("state-link")?;
    let outside = fixture.root().join("outside-state");
    fs::create_dir_all(&outside)?;
    fs::remove_dir_all(fixture.module_root().join("state"))?;
    unix_fs::symlink(&outside, fixture.module_root().join("state"))?;
    let output = run_puread_with_profile_test(restore_args(&fixture), &fixture)?;
    assert_failed_without_runner_mutation(&output, &fixture)
}

#[cfg(unix)]
pub(crate) fn profile_ledger_symlink_is_rejected_before_mutation() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("ledger-link")?;
    let escape = fixture.root().join("escape-ledger");
    fs::write(&escape, appop_record())?;
    fs::remove_file(fixture.profile_ledger_path())?;
    unix_fs::symlink(&escape, fixture.profile_ledger_path())?;
    let output = run_puread_with_profile_test(restore_args(&fixture), &fixture)?;
    assert_failed_without_runner_mutation(&output, &fixture)
}

#[cfg(unix)]
pub(crate) fn run_symlink_is_rejected_before_mutation() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("run-link")?;
    fs::write(fixture.profile_ledger_path(), appop_record())?;
    let outside = fixture.root().join("outside-run");
    fs::create_dir_all(&outside)?;
    fs::remove_dir_all(fixture.module_root().join("run"))?;
    unix_fs::symlink(&outside, fixture.module_root().join("run"))?;
    let output = run_puread_with_profile_test(restore_args(&fixture), &fixture)?;
    assert_failed_without_runner_mutation(&output, &fixture)
}

#[cfg(unix)]
pub(crate) fn lock_symlink_is_rejected_before_mutation() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("lock-link")?;
    fs::write(fixture.profile_ledger_path(), appop_record())?;
    let escape = fixture.root().join("escape-lock");
    fs::write(&escape, "")?;
    fs::remove_file(fixture.module_root().join("run/puread.lock"))?;
    unix_fs::symlink(&escape, fixture.module_root().join("run/puread.lock"))?;
    let output = run_puread_with_profile_test(restore_args(&fixture), &fixture)?;
    assert_failed_without_runner_mutation(&output, &fixture)
}

pub(crate) fn execute_lock_contention_prevents_ledger_read() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("lock-contention")?;
    let lock_guard = fixture.hold_global_lock()?;
    fs::remove_file(fixture.profile_ledger_path())?;
    let output = run_puread_with_profile_test(restore_args(&fixture), &fixture)?;
    assert_lock_failed_without_runner_mutation(&output, &fixture)?;
    drop(lock_guard);
    Ok(())
}

fn restore_args(fixture: &TempFixture) -> [&str; 7] {
    [
        "profile-restore",
        "--execute",
        "--module-root",
        fixture.module_root_str(),
        "--test-profile-runner",
        "--profile-runner-log",
        fixture.runner_log_str(),
    ]
}

#[derive(Debug)]
pub(crate) struct TempFixture {
    root: PathBuf,
    module_root: PathBuf,
    module_root_arg: String,
    runner_log_arg: String,
}

impl TempFixture {
    pub(crate) fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("puread-cli-profile-{name}-{nanos}"));
        let module_root = root.join("module");
        fs::create_dir_all(module_root.join("state/profile-backups"))?;
        fs::create_dir_all(module_root.join("run"))?;
        fs::create_dir_all(root.join("data/system/users/0"))?;
        fs::write(module_root.join("run/puread.lock"), "")?;
        fs::write(module_root.join("state/profile-actions.jsonl"), "")?;
        let module_root_arg = module_root.to_string_lossy().into_owned();
        let runner_log_arg = root
            .join("profile-runner.log")
            .to_string_lossy()
            .into_owned();
        Ok(Self {
            root,
            module_root,
            module_root_arg,
            runner_log_arg,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn module_root(&self) -> &Path {
        &self.module_root
    }

    pub(crate) fn module_root_str(&self) -> &str {
        &self.module_root_arg
    }

    pub(crate) fn profile_ledger_path(&self) -> PathBuf {
        self.module_root.join("state/profile-actions.jsonl")
    }

    pub(crate) fn runner_log(&self) -> PathBuf {
        self.root.join("profile-runner.log")
    }

    pub(crate) fn runner_log_str(&self) -> &str {
        &self.runner_log_arg
    }

    fn hold_global_lock(&self) -> Result<fs::File, Box<dyn Error>> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.module_root.join("run/puread.lock"))?;
        file.try_lock_exclusive()?;
        Ok(file)
    }

    pub(crate) fn shared_prefs_path(&self) -> PathBuf {
        self.root
            .join("data/system/users/0/package-restrictions.xml")
    }

    fn shared_prefs_backup_path(&self) -> PathBuf {
        self.module_root
            .join("state/profile-backups/miui-weather-content-promotion.xml.bak")
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _remove_result = fs::remove_dir_all(&self.root);
    }
}
