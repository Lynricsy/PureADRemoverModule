#![doc = "T20 CLI 手动扫描和应用 profile 行为测试。"]

use std::error::Error;
use std::fs;
use std::path::Path;

#[path = "support/cli_profiles.rs"]
pub(crate) mod support;

use support::{
    ANDROID_FS_FIXTURE, LEDGER_FIXTURE, TempFixture, appops_rules, assert_success, component_rules,
    field, fixture_listing, parse_stdout_json, rom_rules, run_puread,
    run_puread_with_failing_profile_ledger, run_puread_with_profile_runner,
};

#[test]
fn cli_profiles_status_outputs_paths_and_lock_state_when_module_root_is_fixture()
-> Result<(), Box<dyn Error>> {
    // Given: a safe module root fixture with no held lock.
    let fixture = TempFixture::new("status")?;

    // When: status is requested through the real CLI surface.
    let output = run_puread([
        "status",
        "--module-root",
        fixture.module_root_str(),
        "--root",
        ANDROID_FS_FIXTURE,
    ])?;

    // Then: stdout contains stable JSON with path and lock state fields.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "command")?, "status");
    assert_eq!(field(&document, "module_root")?, fixture.module_root_str());
    assert_eq!(field(&document, "root_path")?, ANDROID_FS_FIXTURE);
    assert_eq!(
        field(&document, "lock_path")?,
        fixture.lock_path().to_string_lossy().as_ref()
    );
    assert_eq!(field(&document, "lock_held")?, false);
    Ok(())
}

#[test]
fn cli_profiles_scan_defaults_to_dry_run_when_flag_is_omitted() -> Result<(), Box<dyn Error>> {
    // Given: the Android filesystem fixture has known ad cache paths.
    let before = fixture_listing(Path::new(ANDROID_FS_FIXTURE))?;

    // When: scan is requested without --execute and without --dry-run.
    let output = run_puread(["scan", "--root", ANDROID_FS_FIXTURE])?;

    // Then: the CLI reports a dry-run plan and does not mutate the fixture.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "mode")?, "dry_run");
    assert_eq!(field(&document, "dry_run")?, true);
    assert_eq!(field(&document, "will_mutate")?, false);
    assert_eq!(field(&document, "action_count")?, 2);
    let after = fixture_listing(Path::new(ANDROID_FS_FIXTURE))?;
    assert_eq!(before, after);
    Ok(())
}

#[test]
fn cli_profiles_apply_profile_defaults_to_dry_run_when_execute_is_omitted()
-> Result<(), Box<dyn Error>> {
    // Given: a safe module root fixture and the conservative profile.
    let fixture = TempFixture::new("apply-dry-run")?;
    let before = fixture_listing(Path::new(ANDROID_FS_FIXTURE))?;

    // When: apply-profile is requested without --execute.
    let output = run_puread([
        "apply-profile",
        "conservative",
        "--root",
        ANDROID_FS_FIXTURE,
        "--module-root",
        fixture.module_root_str(),
    ])?;

    // Then: it plans actions without acquiring a mutation lock or changing files.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "mode")?, "dry_run");
    assert_eq!(field(&document, "profile")?, "conservative");
    assert_eq!(field(&document, "will_mutate")?, false);
    assert_eq!(field(&document, "lock_acquired")?, false);
    let after = fixture_listing(Path::new(ANDROID_FS_FIXTURE))?;
    assert_eq!(before, after);
    Ok(())
}

#[test]
fn cli_profiles_apply_profile_execute_uses_global_lock_under_fixture() -> Result<(), Box<dyn Error>>
{
    // Given: a copied Android filesystem and safe module root.
    let fixture = TempFixture::new("apply-execute")?;
    let root = fixture.copy_android_fs()?;
    let root_arg = root.to_string_lossy().into_owned();

    // When: apply-profile is executed explicitly.
    let output = run_puread([
        "apply-profile",
        "conservative",
        "--execute",
        "--root",
        root_arg.as_str(),
        "--module-root",
        fixture.module_root_str(),
    ])?;

    // Then: the lock path is used, the command mutates only fixture paths, and reports success.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "mode")?, "execute");
    assert_eq!(field(&document, "will_mutate")?, true);
    assert_eq!(field(&document, "lock_acquired")?, true);
    assert_eq!(
        field(&document, "lock_path")?,
        fixture.lock_path().to_string_lossy().as_ref()
    );
    assert_eq!(field(&document, "failed")?, 0);
    assert!(fixture.lock_path().is_file());
    let target = root.join("sdcard/Android/data/com.ss.android.ugc.aweme/splashCache");
    assert!(target.is_dir());
    assert!(fs::read_dir(target)?.next().is_none());
    Ok(())
}

#[test]
fn cli_profiles_explicit_profiles_plan_disabled_android_rules() -> Result<(), Box<dyn Error>> {
    // Given: appops/component/rom rules are explicit and disabled by default.
    let fixture = TempFixture::new("explicit-profile-plan")?;

    // When / Then: each explicit profile still selects its matching rule set.
    for (profile, rules, expected_count) in [
        ("appops", appops_rules(), 2),
        ("component", component_rules(), 4),
        ("rom", rom_rules(), 3),
    ] {
        let output = run_puread([
            "apply-profile",
            profile,
            "--rules",
            rules,
            "--root",
            ANDROID_FS_FIXTURE,
            "--module-root",
            fixture.module_root_str(),
        ])?;
        assert_success(&output)?;
        let document = parse_stdout_json(&output)?;
        assert_eq!(field(&document, "mode")?, "dry_run");
        assert_eq!(field(&document, "profile")?, profile);
        assert_eq!(field(&document, "action_count")?, expected_count);
    }
    Ok(())
}

#[test]
fn cli_profiles_execute_dispatches_android_profiles_with_injected_runner()
-> Result<(), Box<dyn Error>> {
    // Given: explicit Android profile rules and an observable test runner seam.
    let fixture = TempFixture::new("explicit-profile-execute")?;

    // When / Then: every explicit Android profile dispatches through the injected runner.
    for (profile, rules, expected) in [
        ("appops", appops_rules(), "appops set"),
        ("component", component_rules(), "pm disable-user"),
        ("rom", rom_rules(), "getprop ro.miui.ui.version.name"),
    ] {
        let runner_log = fixture.profile_runner_log();
        let output = run_puread_with_profile_runner(
            [
                "apply-profile",
                profile,
                "--execute",
                "--rules",
                rules,
                "--root",
                ANDROID_FS_FIXTURE,
                "--module-root",
                fixture.module_root_str(),
            ],
            &runner_log,
        )?;
        assert_success(&output)?;
        let document = parse_stdout_json(&output)?;
        assert_eq!(field(&document, "mode")?, "execute");
        assert_eq!(field(&document, "failed")?, 0);
        assert!(field(&document, "action_count")?.as_u64().unwrap_or(0) > 0);
        assert!(fs::read_to_string(&runner_log)?.contains(expected));
    }
    assert!(fs::read_to_string(fixture.profile_ledger_path())?.contains("\"kind\""));
    Ok(())
}

#[test]
fn cli_profiles_profile_ledger_failure_blocks_android_mutation() -> Result<(), Box<dyn Error>> {
    // Given: an injected runner and a failing persistent profile ledger sink.
    let fixture = TempFixture::new("profile-ledger-fail")?;
    let runner_log = fixture.profile_runner_log();

    // When: component execution reaches ledger append.
    let output = run_puread_with_failing_profile_ledger(
        [
            "apply-profile",
            "component",
            "--execute",
            "--rules",
            component_rules(),
            "--root",
            ANDROID_FS_FIXTURE,
            "--module-root",
            fixture.module_root_str(),
        ],
        &runner_log,
    )?;

    // Then: read-only probes can run, but pm hide and disable-user mutations are absent.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "mode")?, "execute");
    assert_eq!(field(&document, "applied")?, 0);
    assert!(field(&document, "failed")?.as_u64().unwrap_or(0) > 0);
    let calls = fs::read_to_string(&runner_log)?;
    assert!(!calls.contains("pm hide"), "{calls}");
    assert!(!calls.contains("pm disable-user"), "{calls}");
    assert!(!fixture.profile_ledger_path().exists());
    Ok(())
}

#[test]
fn cli_profiles_apply_profile_execute_reports_lock_conflict_when_lock_is_held()
-> Result<(), Box<dyn Error>> {
    // Given: an existing lock file that another process holds.
    let fixture = TempFixture::new("apply-lock-conflict")?;
    let lock_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(fixture.lock_path())?;
    fs2::FileExt::try_lock_exclusive(&lock_file)?;

    // When: apply-profile --execute is requested with the same lock path.
    let output = run_puread([
        "apply-profile",
        "conservative",
        "--execute",
        "--root",
        ANDROID_FS_FIXTURE,
        "--module-root",
        fixture.module_root_str(),
    ])?;

    // Then: the CLI fails clearly before mutation.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("global lock is already held"), "{stderr}");
    Ok(())
}

#[test]
fn cli_profiles_restore_keeps_existing_dry_run_contract() -> Result<(), Box<dyn Error>> {
    // Given: a valid restore ledger fixture.
    let before = fs::read(LEDGER_FIXTURE)?;

    // When: restore is run through the top-level T20 command.
    let output = run_puread(["restore", "--dry-run", "--ledger", LEDGER_FIXTURE])?;

    // Then: it still emits the T12 dry-run report and preserves the ledger.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "mode")?, "dry_run");
    assert_eq!(field(&document, "will_mutate")?, false);
    assert_eq!(field(&document, "action_count")?, 2);
    assert_eq!(before, fs::read(LEDGER_FIXTURE)?);
    Ok(())
}

#[test]
fn cli_profiles_dump_report_uses_existing_ledger_report_without_mutation()
-> Result<(), Box<dyn Error>> {
    // Given: a valid restore ledger fixture.
    let before = fs::read(LEDGER_FIXTURE)?;

    // When: dump-report is requested.
    let output = run_puread(["dump-report", "--ledger", LEDGER_FIXTURE])?;

    // Then: it emits a stable report envelope without modifying the ledger.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "command")?, "dump_report");
    assert_eq!(field(&document, "ledger_path")?, LEDGER_FIXTURE);
    assert_eq!(field(&document, "record_count")?, 2);
    assert_eq!(before, fs::read(LEDGER_FIXTURE)?);
    Ok(())
}
