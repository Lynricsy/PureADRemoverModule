#![doc = "规则 CLI 行为测试。"]

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

const COMMON_RULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/common");
const APP_RULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/apps");
const SQLITE_RULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/sqlite");
const MISSING_RULES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/missing-rules"
);
static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn rules_validate_reports_summary_when_file_rules_are_valid() -> Result<(), Box<dyn Error>> {
    // Given: the default file and SDK cache rule directories.
    // When: validation is requested through the CLI.
    let output = run_puread(["rules", "validate", COMMON_RULES, APP_RULES])?;

    // Then: stdout contains a stable validation summary.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(json_field(&document, "schema_version")?, 1);
    assert_eq!(json_field(&document, "command")?, "rules_validate");
    assert_eq!(json_field(&document, "valid")?, true);
    assert_eq!(json_field(&document, "error_count")?, 0);
    assert_positive_usize(json_field(&document, "rule_file_count")?)?;
    assert_positive_usize(json_field(&document, "rule_count")?)?;
    Ok(())
}

#[test]
fn rules_validate_reports_summary_when_sqlite_rules_are_valid() -> Result<(), Box<dyn Error>> {
    // Given: the SQLite rule directory converted from upstream local database rules.
    // When: validation is requested through the CLI.
    let output = run_puread(["rules", "validate", SQLITE_RULES])?;

    // Then: stdout reports the SQLite rules as valid without mutating anything.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(json_field(&document, "valid")?, true);
    assert_eq!(json_field(&document, "error_count")?, 0);
    assert_positive_usize(json_field(&document, "rule_file_count")?)?;
    assert_positive_usize(json_field(&document, "rule_count")?)?;
    Ok(())
}

#[test]
fn rules_list_files_outputs_only_file_and_sdk_cache_rules() -> Result<(), Box<dyn Error>> {
    // Given: the repository rules directory.
    // When: file-like rules are listed through the CLI.
    let output = run_puread(["rules", "list", "--kind", "files"])?;

    // Then: only file_path and sdk_cache categories are returned.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(json_field(&document, "schema_version")?, 1);
    assert_eq!(json_field(&document, "command")?, "rules_list");
    assert_eq!(json_field(&document, "kind")?, "files");
    let rules = rules_array(&document)?;
    assert!(
        rules
            .iter()
            .any(|rule| json_string_field_eq(rule, "category", "file_path"))
    );
    assert!(
        rules
            .iter()
            .any(|rule| json_string_field_eq(rule, "category", "sdk_cache"))
    );
    assert!(rules.iter().all(|rule| matches!(
        json_field(rule, "category"),
        Ok(Value::String(category)) if category == "file_path" || category == "sdk_cache"
    )));
    Ok(())
}

#[test]
fn rules_list_sqlite_outputs_only_sqlite_rules() -> Result<(), Box<dyn Error>> {
    // Given: the repository rules directory.
    // When: SQLite rules are listed through the CLI.
    let output = run_puread(["rules", "list", "--kind", "sqlite"])?;

    // Then: every returned rule is in the sqlite category.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(json_field(&document, "kind")?, "sqlite");
    let rules = rules_array(&document)?;
    assert!(!rules.is_empty());
    assert!(
        rules
            .iter()
            .all(|rule| json_field(rule, "category").is_ok_and(|category| category == "sqlite"))
    );
    Ok(())
}

#[test]
fn rules_validate_fails_cleanly_when_rules_path_is_missing() -> Result<(), Box<dyn Error>> {
    // Given: a missing rules path.
    // When: validation is requested.
    let output = run_puread(["rules", "validate", MISSING_RULES])?;

    // Then: the CLI exits non-zero and reports the missing path.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("rules path does not exist"), "{stderr}");
    Ok(())
}

#[test]
fn json_field_is_zero_accepts_zero_integer_field() -> Result<(), Box<dyn Error>> {
    // Given: an auto-apply summary JSON file whose failed count is zero.
    let path = temp_json_path("zero-field")?;
    fs::write(&path, r#"{"failed":0,"applied":3}"#)?;

    // When: the field guard is executed through the real CLI surface.
    let output = run_puread([
        "json-field-is-zero",
        "--file",
        path.to_string_lossy().as_ref(),
        "--field",
        "failed",
    ])?;

    // Then: the guard exits successfully for service.sh to mark auto-apply complete.
    assert_success(&output)?;
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn json_field_is_zero_rejects_nonzero_or_missing_field() -> Result<(), Box<dyn Error>> {
    // Given: one summary has failures and another summary lacks the expected field.
    let nonzero = temp_json_path("nonzero-field")?;
    let missing = temp_json_path("missing-field")?;
    fs::write(&nonzero, r#"{"failed":1}"#)?;
    fs::write(&missing, r#"{"applied":1}"#)?;

    // When: each file is checked through the real CLI guard.
    let nonzero_output = run_puread([
        "json-field-is-zero",
        "--file",
        nonzero.to_string_lossy().as_ref(),
        "--field",
        "failed",
    ])?;
    let missing_output = run_puread([
        "json-field-is-zero",
        "--file",
        missing.to_string_lossy().as_ref(),
        "--field",
        "failed",
    ])?;

    // Then: service.sh receives a non-zero exit code and does not mark success.
    assert!(!nonzero_output.status.success(), "{nonzero_output:?}");
    assert!(!missing_output.status.success(), "{missing_output:?}");
    fs::remove_file(nonzero)?;
    fs::remove_file(missing)?;
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

fn assert_positive_usize(value: &Value) -> Result<(), Box<dyn Error>> {
    let Some(number) = value.as_u64() else {
        return Err("value must be an unsigned integer".into());
    };
    if number == 0 {
        return Err("value must be positive".into());
    }
    Ok(())
}

fn json_field<'a>(document: &'a Value, key: &str) -> Result<&'a Value, Box<dyn Error>> {
    document
        .get(key)
        .ok_or_else(|| format!("missing JSON field: {key}").into())
}

fn json_string_field_eq(document: &Value, key: &str, expected: &str) -> bool {
    json_field(document, key).is_ok_and(|value| value == expected)
}

fn rules_array(document: &Value) -> Result<&[Value], Box<dyn Error>> {
    let Some(rules) = json_field(document, "rules")?.as_array() else {
        return Err("rules must be a JSON array".into());
    };
    Ok(rules)
}

fn temp_json_path(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "puread-cli-{name}-{}-{id}.json",
        std::process::id()
    ));
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(path)
}
