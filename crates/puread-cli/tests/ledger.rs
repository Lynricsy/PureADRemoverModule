#![doc = "恢复账本 CLI 行为测试。"]

use std::error::Error;
use std::fs;
use std::process::{Command, Output};

const LEDGER_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/ledger.json"
);
const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");
const MALFORMED_LEDGER_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/malformed-ledger.json"
);

#[test]
fn ledger_show_outputs_stable_json_when_ledger_exists() -> Result<(), Box<dyn Error>> {
    // Given: a valid restore ledger fixture.
    // When: the ledger is shown through the CLI.
    let output = run_puread(["ledger", "show", "--ledger", LEDGER_FIXTURE])?;

    // Then: stdout contains the stable ledger JSON envelope.
    assert_success(&output)?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains(r#""schema_version": 1"#), "{stdout}");
    assert!(stdout.contains(r#""record_count": 2"#), "{stdout}");
    assert!(stdout.contains(r#""records""#), "{stdout}");
    Ok(())
}

#[test]
fn restore_dry_run_outputs_restore_actions_when_ledger_exists() -> Result<(), Box<dyn Error>> {
    // Given: a valid restore ledger fixture with a parent and child path.
    // When: restore planning is requested in dry-run mode.
    let output = run_puread(["restore", "--dry-run", "--ledger", LEDGER_FIXTURE])?;

    // Then: the CLI reports planned restore actions without claiming mutation.
    assert_success(&output)?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains(r#""mode": "dry_run""#), "{stdout}");
    assert!(stdout.contains(r#""action_count": 2"#), "{stdout}");
    assert!(stdout.contains(r#""will_mutate": false"#), "{stdout}");
    assert!(
        stdout.contains("/data/data/com.demo/cache/ad.bin"),
        "{stdout}"
    );
    Ok(())
}

#[test]
fn restore_dry_run_preserves_ledger_fixture_when_planning_restore() -> Result<(), Box<dyn Error>> {
    // Given: the fixture bytes before dry-run planning.
    let before = fs::read(LEDGER_FIXTURE)?;

    // When: restore dry-run is executed.
    let output = run_puread(["restore", "--dry-run", "--ledger", LEDGER_FIXTURE])?;

    // Then: the command succeeds and the ledger file remains byte-for-byte unchanged.
    assert_success(&output)?;
    let after = fs::read(LEDGER_FIXTURE)?;
    assert_eq!(before, after);
    Ok(())
}

#[test]
fn restore_dry_run_fails_cleanly_when_ledger_path_is_missing() -> Result<(), Box<dyn Error>> {
    // Given: a missing ledger path.
    // When: restore dry-run is requested.
    let output = run_puread([
        "restore",
        "--dry-run",
        "--ledger",
        "tests/fixtures/missing-ledger.json",
    ])?;

    // Then: the CLI exits non-zero and reports the missing ledger path.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("ledger file does not exist"), "{stderr}");
    Ok(())
}

#[test]
fn ledger_show_fails_cleanly_when_ledger_path_is_directory() -> Result<(), Box<dyn Error>> {
    // Given: a directory path where a ledger file is required.
    // When: the ledger is shown through the CLI.
    let output = run_puread(["ledger", "show", "--ledger", FIXTURE_DIR])?;

    // Then: the CLI exits non-zero and reports the malformed path.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("ledger path is not a file"), "{stderr}");
    Ok(())
}

#[test]
fn restore_dry_run_fails_cleanly_when_ledger_json_is_malformed() -> Result<(), Box<dyn Error>> {
    // Given: a malformed JSONL ledger fixture.
    // When: restore dry-run is requested.
    let output = run_puread(["restore", "--dry-run", "--ledger", MALFORMED_LEDGER_FIXTURE])?;

    // Then: the CLI exits non-zero and reports the bad ledger line.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("ledger JSONL line 1 is invalid"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn restore_without_dry_run_fails_without_mutating_ledger_fixture() -> Result<(), Box<dyn Error>> {
    // Given: a valid ledger fixture and a restore command without dry-run.
    let before = fs::read(LEDGER_FIXTURE)?;

    // When: real restore is requested.
    let output = run_puread(["restore", "--ledger", LEDGER_FIXTURE])?;

    // Then: the CLI rejects real execution and leaves the ledger unchanged.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("real restore execution is not implemented"),
        "{stderr}"
    );
    let after = fs::read(LEDGER_FIXTURE)?;
    assert_eq!(before, after);
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
