#![doc = "扫描 dry-run CLI 行为测试。"]

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

const ANDROID_FS_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/android-fs"
);
const MISSING_RULES_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/missing-rules"
);
const MISSING_ROOT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/missing-android-fs"
);

#[test]
fn cli_dry_run_outputs_stable_json_when_file_rules_match_fixture() -> Result<(), Box<dyn Error>> {
    // Given: a fake Android filesystem with two known app-local ad cache paths.
    // When: scan planning is requested through the real CLI surface.
    let output = run_puread([
        "scan",
        "--dry-run",
        "--rules",
        "rules/files",
        "--root",
        ANDROID_FS_FIXTURE,
    ])?;

    // Then: stdout is stable JSON containing planned path/action entries only.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(json_field(&document, "schema_version")?, 1);
    assert_eq!(json_field(&document, "mode")?, "dry_run");
    assert_eq!(json_field(&document, "dry_run")?, true);
    assert_eq!(json_field(&document, "will_mutate")?, false);
    assert_eq!(json_field(&document, "action_count")?, 2);

    let Some(actions) = json_field(&document, "actions")?.as_array() else {
        return Err("actions must be a JSON array".into());
    };
    assert!(actions.iter().any(|action| {
        json_string_field_eq(action, "rule_id", "aweme-external-splash-cache")
            && json_string_field_eq(action, "action", "empty_dir")
            && json_string_field_eq(
                action,
                "android_path",
                "/sdcard/Android/data/com.ss.android.ugc.aweme/splashCache",
            )
    }));
    assert!(actions.iter().any(|action| {
        json_string_field_eq(action, "rule_id", "aweme-internal-splash-commerce-card")
            && json_string_field_eq(action, "action", "empty_dir")
            && json_string_field_eq(
                action,
                "android_path",
                "/data/user/0/com.ss.android.ugc.aweme/cache/splash_commerce_card",
            )
    }));
    Ok(())
}

#[test]
fn cli_dry_run_preserves_android_fixture_when_planning_scan() -> Result<(), Box<dyn Error>> {
    // Given: a recursive listing of the fake Android filesystem before planning.
    let before = fixture_listing(Path::new(ANDROID_FS_FIXTURE))?;

    // When: scan dry-run is executed.
    let output = run_puread([
        "scan",
        "--dry-run",
        "--rules",
        "rules/files",
        "--root",
        ANDROID_FS_FIXTURE,
    ])?;

    // Then: the command succeeds and the fixture remains byte-for-byte equivalent.
    assert_success(&output)?;
    let after = fixture_listing(Path::new(ANDROID_FS_FIXTURE))?;
    assert_eq!(before, after);
    Ok(())
}

#[test]
fn cli_dry_run_fails_cleanly_when_rules_path_is_missing() -> Result<(), Box<dyn Error>> {
    // Given: a missing rules path.
    // When: scan dry-run is requested.
    let output = run_puread([
        "scan",
        "--dry-run",
        "--rules",
        MISSING_RULES_PATH,
        "--root",
        ANDROID_FS_FIXTURE,
    ])?;

    // Then: the CLI exits non-zero and reports the missing rule path.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("rules path does not exist"), "{stderr}");
    Ok(())
}

#[test]
fn cli_dry_run_fails_cleanly_when_root_path_is_missing() -> Result<(), Box<dyn Error>> {
    // Given: a missing fake Android root path.
    // When: scan dry-run is requested.
    let output = run_puread([
        "scan",
        "--dry-run",
        "--rules",
        "rules/files",
        "--root",
        MISSING_ROOT_PATH,
    ])?;

    // Then: the CLI exits non-zero and reports the missing root path.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("root path does not exist"), "{stderr}");
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

fn json_field<'a>(document: &'a Value, key: &str) -> Result<&'a Value, Box<dyn Error>> {
    document
        .get(key)
        .ok_or_else(|| format!("missing JSON field: {key}").into())
}

fn json_string_field_eq(document: &Value, key: &str, expected: &str) -> bool {
    json_field(document, key).is_ok_and(|value| value == expected)
}

fn fixture_listing(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
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
