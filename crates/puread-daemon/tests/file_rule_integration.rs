#![doc = "`puread-daemon` 文件规则 dry-run 集成测试。"]

#[path = "file_rule_integration/fixture.rs"]
pub mod fixture;
#[path = "file_rule_integration/harness.rs"]
pub mod harness;

use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use puread_core::model::RuleAction;
use puread_core::restore_ledger::{LedgerAction, RestoreLedger};
use puread_daemon::{
    DaemonError, DaemonEvent, EventLoop, FileRuleDaemonConfig, FileRuleDaemonMode,
};
use puread_rules::RuleCategory;

use fixture::mixed_rules;
use harness::{TestTempDir, recv_matching, worker_result};

const EVENT_TIMEOUT: Duration = Duration::from_secs(3);
const DEBOUNCE: Duration = Duration::from_millis(40);

#[test]
fn file_rule_integration_outputs_dry_run_plan_when_splash_cache_is_created()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a dry-run daemon loads mixed rules and watches an Android-like package tree.
    let temp = TestTempDir::new("puread-daemon-file-rule")?;
    let android_root = temp.path().join("android-root");
    let rule_dir = temp.path().join("rules");
    let package_root = android_root.join("sdcard/Android/data/com.example.video");
    std::fs::create_dir_all(&package_root)?;
    std::fs::create_dir_all(&rule_dir)?;
    std::fs::write(rule_dir.join("mixed.toml"), mixed_rules())?;
    let config = FileRuleDaemonConfig::new(
        android_root,
        vec![rule_dir],
        FileRuleDaemonMode::DryRun,
        DEBOUNCE,
    )?;
    let (mut event_loop, handle) = EventLoop::from_file_rules(&config)?;
    let (started_tx, started_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        event_loop.run(|event| {
            if matches!(event, DaemonEvent::Started) {
                started_tx
                    .send(())
                    .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            }
            event_tx
                .send(event)
                .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            Ok(())
        })
    });

    started_rx.recv_timeout(EVENT_TIMEOUT)?;

    // When: the watched splash cache directory is recreated by the app.
    let splash_cache = package_root.join("splashCache");
    std::fs::create_dir(&splash_cache)?;

    // Then: the daemon emits a dry-run plan and does not mutate the filesystem.
    let event = recv_matching(&event_rx, is_plan_event, EVENT_TIMEOUT)?;
    let DaemonEvent::DryRunFilePlan { actions } = event else {
        return Err("expected dry-run file plan event".into());
    };
    assert_eq!(actions.len(), 1);
    let action = actions.first().ok_or("expected one dry-run file action")?;
    assert_eq!(action.rule_id(), "video-splash-cache");
    assert_eq!(action.category(), RuleCategory::FilePath);
    assert_eq!(action.action(), RuleAction::EmptyDir);
    assert_eq!(
        action.android_path(),
        Path::new("/sdcard/Android/data/com.example.video/splashCache")
    );
    assert_eq!(action.host_path(), splash_cache.as_path());
    assert!(splash_cache.is_dir());

    handle.shutdown()?;
    worker_result(worker, EVENT_TIMEOUT)??;
    Ok(())
}

#[test]
fn file_rule_integration_apply_mode_executes_file_action_and_writes_ledger()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: apply mode loads default-enabled file rules and a writable ledger path.
    let temp = TestTempDir::new("puread-daemon-file-rule-apply")?;
    let android_root = temp.path().join("android-root");
    let rule_dir = temp.path().join("rules");
    let ledger_path = temp.path().join("state/actions.jsonl");
    let package_root = android_root.join("sdcard/Android/data/com.example.video");
    std::fs::create_dir_all(&package_root)?;
    std::fs::create_dir_all(&rule_dir)?;
    std::fs::write(rule_dir.join("mixed.toml"), mixed_rules())?;
    let config = FileRuleDaemonConfig::new(
        android_root,
        vec![rule_dir],
        FileRuleDaemonMode::Apply {
            ledger_path: ledger_path.clone(),
        },
        DEBOUNCE,
    )?;
    let (mut event_loop, handle) = EventLoop::from_file_rules(&config)?;
    let (started_tx, started_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        event_loop.run(|event| {
            if matches!(event, DaemonEvent::Started) {
                started_tx
                    .send(())
                    .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            }
            event_tx
                .send(event)
                .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            Ok(())
        })
    });

    started_rx.recv_timeout(EVENT_TIMEOUT)?;

    // When: the app recreates a matching splash cache containing an ad marker.
    let splash_cache = package_root.join("splashCache");
    std::fs::create_dir(&splash_cache)?;
    std::fs::write(splash_cache.join("ad.tmp"), b"ad")?;

    // Then: apply mode executes the file action, emits an outcome, and writes ledger.
    let event = recv_matching(&event_rx, is_apply_event, EVENT_TIMEOUT)?;
    let DaemonEvent::FileRuleApplyReport { outcomes } = event else {
        return Err("expected file-rule apply report event".into());
    };
    assert_eq!(outcomes.len(), 1);
    let outcome = outcomes.first().ok_or("expected one apply outcome")?;
    assert_eq!(outcome.rule_id(), "video-splash-cache");
    assert!(outcome.will_mutate());
    assert!(!splash_cache.join("ad.tmp").exists());
    let records = RestoreLedger::at(ledger_path).read_records()?;
    assert_eq!(records.len(), 1);
    let record = records.first().ok_or("expected one ledger record")?;
    assert_eq!(record.action, LedgerAction::EmptyDir);

    handle.shutdown()?;
    worker_result(worker, EVENT_TIMEOUT)??;
    Ok(())
}

#[test]
fn file_rule_integration_excludes_sqlite_appops_and_component_from_watch_roots()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: mixed rule documents contain one file rule and three non-file rules.
    let temp = TestTempDir::new("puread-daemon-file-rule-filter")?;
    let android_root = temp.path().join("android-root");
    let rule_dir = temp.path().join("rules");
    std::fs::create_dir_all(android_root.join("sdcard/Android/data/com.example.video"))?;
    std::fs::create_dir_all(android_root.join("data/data/com.example.video/databases"))?;
    std::fs::create_dir_all(&rule_dir)?;
    std::fs::write(rule_dir.join("mixed.toml"), mixed_rules())?;

    // When: dry-run daemon config is prepared from rule files.
    let config = FileRuleDaemonConfig::new(
        android_root,
        vec![rule_dir],
        FileRuleDaemonMode::DryRun,
        DEBOUNCE,
    )?;
    let runtime = config.prepare()?;

    // Then: only the file_path watch root is registered for high-frequency watching.
    let roots = runtime.watch_roots();
    assert_eq!(roots.len(), 1);
    let root = roots.first().ok_or("expected one file watch root")?;
    assert!(root.ends_with("sdcard/Android/data/com.example.video"));
    assert!(
        roots
            .iter()
            .all(|path| !path.to_string_lossy().contains("databases"))
    );
    assert_eq!(runtime.file_rule_count(), 1);
    assert_eq!(runtime.skipped_high_frequency_rule_count(), 3);
    Ok(())
}

const fn is_plan_event(event: &DaemonEvent) -> bool {
    matches!(event, DaemonEvent::DryRunFilePlan { .. })
}

const fn is_apply_event(event: &DaemonEvent) -> bool {
    matches!(event, DaemonEvent::FileRuleApplyReport { .. })
}
