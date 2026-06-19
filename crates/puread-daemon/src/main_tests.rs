use std::path::PathBuf;
use std::time::Duration;

use puread_daemon::FileRuleDaemonMode;

use super::{CliArgError, DaemonArgs};

#[test]
fn daemon_args_parse_apply_mode_with_explicit_ledger_and_debounce() {
    let args = [
        "--apply",
        "--root",
        "/",
        "--rules",
        "/data/adb/modules/PureAD/rules",
        "--state-dir",
        "/data/adb/modules/PureAD/state",
        "--ledger",
        "/data/adb/modules/PureAD/state/actions.jsonl",
        "--log-file",
        "/data/adb/modules/PureAD/logs/puread.log",
        "--debounce-ms",
        "250",
    ]
    .into_iter()
    .map(str::to_owned);

    let parsed = DaemonArgs::parse_from(args).expect("service args should parse");

    assert_eq!(parsed.root, PathBuf::from("/"));
    assert_eq!(
        parsed.rules,
        PathBuf::from("/data/adb/modules/PureAD/rules")
    );
    assert_eq!(
        parsed.log_file,
        Some(PathBuf::from("/data/adb/modules/PureAD/logs/puread.log"))
    );
    assert_eq!(parsed.debounce, Duration::from_millis(250));
    assert_eq!(
        parsed.mode,
        FileRuleDaemonMode::Apply {
            ledger_path: PathBuf::from("/data/adb/modules/PureAD/state/actions.jsonl"),
        }
    );
}

#[test]
fn daemon_args_parse_dry_run_defaults_ledger_under_state_dir() {
    let args = [
        "--dry-run",
        "--root",
        "/tmp/root",
        "--rules",
        "/tmp/rules",
        "--state-dir",
        "/tmp/state",
    ]
    .into_iter()
    .map(str::to_owned);

    let parsed = DaemonArgs::parse_from(args).expect("dry-run args should parse");

    assert_eq!(parsed.root, PathBuf::from("/tmp/root"));
    assert_eq!(parsed.rules, PathBuf::from("/tmp/rules"));
    assert_eq!(parsed.mode, FileRuleDaemonMode::DryRun);
}

#[test]
fn daemon_args_reject_conflicting_modes_unknown_flags_and_zero_debounce() {
    assert!(matches!(
        DaemonArgs::parse_from(["--apply", "--dry-run"].into_iter().map(str::to_owned)),
        Err(CliArgError::ConflictingMode)
    ));
    assert!(matches!(
        DaemonArgs::parse_from(std::iter::once("--unknown").map(str::to_owned)),
        Err(CliArgError::UnknownFlag(flag)) if flag == "--unknown"
    ));
    assert!(matches!(
        DaemonArgs::parse_from(["--debounce-ms", "0"].into_iter().map(str::to_owned)),
        Err(CliArgError::InvalidDebounce)
    ));
}
