#![doc = "CLI `SQLite` profile 执行接入测试。"]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const SQLITE_RULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/sqlite");

#[test]
fn cli_sqlite_profile_execute_runs_sqlite_runner_when_database_exists() -> Result<(), Box<dyn Error>>
{
    // Given: a copied Android root contains a 123pan advertising SQLite database.
    let fixture = TempFixture::new("sqlite-profile")?;
    let root = fixture.android_root()?;
    let db_path = root.join("data/user/0/com.mfcloudcalculate.networkdisk/databases/amps_ad.db");
    write_sqlite_fixture(&db_path)?;
    let before = fs::read(&db_path)?;
    let root_arg = root.to_string_lossy().into_owned();

    // When: the real CLI executes the sqlite profile.
    let output = run_puread([
        "apply-profile",
        "sqlite",
        "--execute",
        "--rules",
        SQLITE_RULES,
        "--root",
        root_arg.as_str(),
        "--module-root",
        fixture.module_root_str(),
    ])?;

    // Then: the database is handled by sqlite_actions, reported as applied, and ledgered.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "mode")?, "execute");
    assert_eq!(field(&document, "profile")?, "sqlite");
    assert!(field(&document, "applied")?.as_u64().unwrap_or(0) > 0);
    assert_eq!(field(&document, "failed")?, 0);
    assert_ne!(fs::read(&db_path)?, before);
    assert_eq!(fs::metadata(&db_path)?.len(), 4096);
    let ledger = fs::read_to_string(fixture.actions_ledger_path())?;
    assert!(ledger.contains("sqlite-123pan-amps-ad-minimal"));
    assert!(ledger.contains("\"original_file_type\":\"file\""));
    assert!(ledger.contains("sqlite:boot_once"));
    Ok(())
}

#[test]
fn cli_sqlite_profile_execute_expands_common_sdk_database_in_any_app_package()
-> Result<(), Box<dyn Error>> {
    // Given: a common advertising SDK database exists in an arbitrary app-private directory.
    let fixture = TempFixture::new("sqlite-any-package")?;
    let root = fixture.android_root()?;
    let db_path = root.join("data/user/0/com.real.app/databases/beizi_ad.db");
    write_sqlite_fixture(&db_path)?;
    let root_arg = root.to_string_lossy().into_owned();

    // When: the real CLI executes the sqlite profile against bundled common SDK rules.
    let output = run_puread([
        "apply-profile",
        "sqlite",
        "--execute",
        "--rules",
        SQLITE_RULES,
        "--root",
        root_arg.as_str(),
        "--module-root",
        fixture.module_root_str(),
    ])?;

    // Then: the sqlite-any marker expands to the real app package and records recovery data.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "failed")?, 0);
    assert!(field(&document, "applied")?.as_u64().unwrap_or(0) > 0);
    assert_eq!(fs::metadata(&db_path)?.len(), 4096);
    let ledger = fs::read_to_string(fixture.actions_ledger_path())?;
    assert!(ledger.contains("sqlite-any-beizi-ad-deny-write"));
    assert!(ledger.contains("/data/user/0/com.real.app/databases/beizi_ad.db"));
    assert!(ledger.contains("sqlite:low_frequency"));
    Ok(())
}

fn run_puread<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-cli"))
        .args(args)
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

fn write_sqlite_fixture(path: &Path) -> Result<(), Box<dyn Error>> {
    let parent = path.parent().ok_or("sqlite fixture path has no parent")?;
    fs::create_dir_all(parent)?;
    fs::write(path, sqlite_image())?;
    Ok(())
}

fn sqlite_image() -> Vec<u8> {
    let mut image = vec![0_u8; 4096];
    write_image_bytes(&mut image, 0, b"SQLite format 3\0");
    write_image_bytes(&mut image, 16, &4096_u16.to_be_bytes());
    write_image_byte(&mut image, 18, 1);
    write_image_byte(&mut image, 19, 1);
    write_image_byte(&mut image, 21, 64);
    write_image_byte(&mut image, 22, 32);
    write_image_byte(&mut image, 23, 32);
    write_image_bytes(&mut image, 28, &1_u32.to_be_bytes());
    write_image_bytes(&mut image, 44, &4_u32.to_be_bytes());
    write_image_byte(&mut image, 100, 0x0d);
    write_image_bytes(&mut image, 105, &4096_u16.to_be_bytes());
    image
}

fn write_image_byte(image: &mut [u8], offset: usize, value: u8) {
    if let Some(byte) = image.get_mut(offset) {
        *byte = value;
    }
}

fn write_image_bytes(image: &mut [u8], offset: usize, value: &[u8]) {
    if let Some(bytes) = image.get_mut(offset..offset.saturating_add(value.len())) {
        bytes.copy_from_slice(value);
    }
}

#[derive(Debug)]
struct TempFixture {
    root: PathBuf,
    module_root_arg: String,
}

impl TempFixture {
    fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("puread-cli-{name}-{nanos}"));
        let module_root = root.join("module");
        fs::create_dir_all(module_root.join("run"))?;
        let module_root_arg = module_root.to_string_lossy().into_owned();
        Ok(Self {
            root,
            module_root_arg,
        })
    }

    fn module_root_str(&self) -> &str {
        &self.module_root_arg
    }

    fn actions_ledger_path(&self) -> PathBuf {
        PathBuf::from(&self.module_root_arg).join("state/actions.jsonl")
    }

    fn android_root(&self) -> Result<PathBuf, Box<dyn Error>> {
        let root = self.root.join("android-fs");
        fs::create_dir_all(root.join("data/user/0"))?;
        Ok(root)
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _remove_result = fs::remove_dir_all(&self.root);
    }
}
