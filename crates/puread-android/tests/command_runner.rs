#![doc = "Android 命令适配层行为测试。"]

use puread_android::command_runner::{
    AndroidCommandAdapter, AppOpsAdapter, ChattrAdapter, ChconAdapter, CommandError, CommandOutput,
    CommandPhase, GetpropAdapter, LsattrAdapter, PmComponentAdapter, SettingsAdapter,
    SettingsNamespace,
};

#[path = "command_runner/support.rs"]
mod support;

use support::{ScriptedRunner, command_lines, fail, ok};

fn assert_invalid<T>(result: Result<T, CommandError>) {
    assert!(result.is_err_and(|error| matches!(error, CommandError::InvalidArgument { .. })));
}

fn assert_all_invalid<T, F>(values: &[&str], build: F)
where
    F: Fn(&str) -> Result<T, CommandError>,
{
    for value in values {
        assert_invalid(build(value));
    }
}

#[test]
fn command_runner_pm_appops_settings_and_getprop_execute_expected_argv_and_record_output()
-> Result<(), CommandError> {
    // Given: adapters for component, appops, ROM setting, and property commands.
    let runner = ScriptedRunner::with_outputs(vec![ok("V14\n"); 10]);
    let pm = PmComponentAdapter::new(0, "com.example/.AdActivity")?;
    let appops = AppOpsAdapter::new("com.example", "RUN_IN_BACKGROUND", "ignore", "default")?;
    let settings = SettingsAdapter::new(
        SettingsNamespace::Secure,
        "miui_home_show_recommend",
        "0",
        Some("1"),
    )?;
    let getprop = GetpropAdapter::new("ro.miui.ui.version.name")?;
    let dry_run = settings.dry_run(CommandPhase::Apply);

    // When: each adapter is driven through probe/apply/restore.
    pm.probe(&runner)?;
    pm.apply(&runner)?;
    pm.restore(&runner)?;
    appops.probe(&runner)?;
    appops.apply(&runner)?;
    appops.restore(&runner)?;
    settings.probe(&runner)?;
    settings.apply(&runner)?;
    settings.restore(&runner)?;
    let property_probe = getprop.probe(&runner)?;

    // Then: the fake runner saw stable argv and successful output is retained.
    assert_eq!(
        property_probe.output().map(CommandOutput::stdout),
        Some("V14\n")
    );
    assert!(dry_run.is_dry_run());
    assert!(dry_run.output().is_none());
    assert_eq!(
        dry_run.invocation().argv(),
        [
            "/system/bin/settings",
            "put",
            "secure",
            "miui_home_show_recommend",
            "0"
        ]
    );
    assert_eq!(
        runner.call_lines(),
        command_lines(
            "/system/bin/pm path com.example\n/system/bin/pm disable-user --user 0 com.example/.AdActivity\n/system/bin/pm enable --user 0 com.example/.AdActivity\n/system/bin/cmd appops get com.example RUN_IN_BACKGROUND\n/system/bin/cmd appops set com.example RUN_IN_BACKGROUND ignore\n/system/bin/cmd appops set com.example RUN_IN_BACKGROUND default\n/system/bin/settings get secure miui_home_show_recommend\n/system/bin/settings put secure miui_home_show_recommend 0\n/system/bin/settings put secure miui_home_show_recommend 1\n/system/bin/getprop ro.miui.ui.version.name"
        )
    );
    Ok(())
}

#[test]
fn command_runner_metadata_adapters_execute_expected_argv() -> Result<(), CommandError> {
    // Given: metadata command adapters and scripted command output.
    let runner = ScriptedRunner::with_outputs(vec![ok(""); 7]);
    let chcon = ChconAdapter::new(
        "/data/user/0/com.example/cache/ad",
        "u:object_r:app_data_file:s0",
        "u:object_r:cache_file:s0",
    )?;
    let chattr = ChattrAdapter::new("/data/user/0/com.example/cache/ad")?;
    let lsattr = LsattrAdapter::new("/data/user/0/com.example/cache/ad")?;

    // When: probe/apply/restore semantics are driven for each adapter.
    chcon.probe(&runner)?;
    chcon.apply(&runner)?;
    chcon.restore(&runner)?;
    chattr.probe(&runner)?;
    chattr.apply(&runner)?;
    chattr.restore(&runner)?;
    lsattr.probe(&runner)?;

    // Then: commands are direct argv vectors, not shell strings.
    assert_eq!(
        runner.call_lines(),
        command_lines(
            "/system/bin/chcon --help\n/system/bin/chcon u:object_r:app_data_file:s0 /data/user/0/com.example/cache/ad\n/system/bin/chcon u:object_r:cache_file:s0 /data/user/0/com.example/cache/ad\n/system/bin/chattr --help\n/system/bin/chattr +i /data/user/0/com.example/cache/ad\n/system/bin/chattr -i /data/user/0/com.example/cache/ad\n/system/bin/lsattr /data/user/0/com.example/cache/ad"
        )
    );
    Ok(())
}

#[test]
fn command_runner_rejects_injected_tokens_without_blocking_android_names()
-> Result<(), CommandError> {
    // Given: representative Android package, component, appops, and property names.
    PmComponentAdapter::new(10, "com.example.app_1/.AdActivity")?;
    PmComponentAdapter::new(0, "com.example/com.example.Outer$Inner")?;
    AppOpsAdapter::new(
        "com.example.app_1",
        "RUN_IN_BACKGROUND",
        "foreground",
        "default",
    )?;
    GetpropAdapter::new("persist.vendor.radio.atfwd.start")?;

    // When/Then: option-looking, path-traversal, whitespace, NUL, and CLI metacharacters fail.
    assert_invalid(ChconAdapter::new(
        "--reference=/data/local/tmp/source",
        "u:object_r:app_data_file:s0",
        "u:object_r:cache_file:s0",
    ));
    assert_all_invalid(&["-R"], ChattrAdapter::new);
    assert_all_invalid(&["--all"], LsattrAdapter::new);
    assert_all_invalid(
        &[
            "--user/.AdActivity",
            "com.example/../Bad",
            "com.example/-x",
            "com example/.A",
            "com.example/.Bad;id",
        ],
        |value| PmComponentAdapter::new(0, value),
    );
    assert_all_invalid(
        &[
            "--user",
            "-x",
            "../pkg",
            "com.example;id",
            "com example",
            "com.example\0bad",
        ],
        |value| AppOpsAdapter::new(value, "RUN_IN_BACKGROUND", "ignore", "default"),
    );
    assert_all_invalid(
        &[
            "--op",
            "RUN;BACKGROUND",
            "run_in_background",
            "RUN/BACKGROUND",
        ],
        |value| AppOpsAdapter::new("com.example", value, "ignore", "default"),
    );
    assert_all_invalid(
        &[
            "--help",
            "-x",
            "../prop",
            "ro.foo;id",
            "ro foo",
            "ro.foo\0bar",
        ],
        GetpropAdapter::new,
    );
    Ok(())
}

#[test]
fn command_runner_read_only_adapters_reject_apply_restore_without_running()
-> Result<(), CommandError> {
    // Given: read-only metadata and property adapters with a runner that would record mutation.
    let runner = ScriptedRunner::default();
    let getprop = GetpropAdapter::new("ro.miui.ui.version.name")?;
    let lsattr = LsattrAdapter::new("/data/user/0/com.example/cache/ad")?;

    // When: apply/restore are requested on read-only adapters.
    for error in [
        getprop.apply(&runner),
        getprop.restore(&runner),
        lsattr.apply(&runner),
        lsattr.restore(&runner),
    ] {
        assert!(matches!(
            error,
            Err(CommandError::UnsupportedReadOnlyPhase { .. })
        ));
    }
    assert!(runner.calls().is_empty());
    Ok(())
}

#[test]
fn command_output_from_status_constructs_success_and_failure_without_naming_bias() {
    // Given/When: process output is built from the actual exit status.
    let success = CommandOutput::from_status(0, "ok", "");
    let failure = CommandOutput::from_status(3, "", "denied");

    // Then: the status and streams are preserved for both outcomes.
    assert_eq!(success.status(), 0);
    assert_eq!(success.stdout(), "ok");
    assert_eq!(failure.status(), 3);
    assert_eq!(failure.stderr(), "denied");
}

#[test]
fn command_runner_command_failure_preserves_status_stdout_stderr_and_argv()
-> Result<(), CommandError> {
    // Given: appops returns a non-zero status with stderr.
    let runner = ScriptedRunner::with_outputs(vec![fail("Unknown operation")]);
    let adapter = AppOpsAdapter::new("com.example", "RUN_IN_BACKGROUND", "ignore", "default")?;

    // When: apply is executed.
    // Then: the error keeps the argv and process output for reports.
    assert!(matches!(
        adapter.apply(&runner),
        Err(CommandError::CommandFailed {
            status: 7,
            ref stderr,
            ref invocation,
            ..
        }) if stderr == "Unknown operation" && invocation.argv()
            == ["/system/bin/cmd", "appops", "set", "com.example", "RUN_IN_BACKGROUND", "ignore"]
    ));
    Ok(())
}

#[test]
fn command_runner_settings_adapter_rejects_denied_network_related_keys_before_runner() {
    // Given/When/Then: unsafe network-control setting keys are rejected at construction.
    for key in [
        "private_dns_mode",
        "private_dns_specifier",
        "http_proxy",
        "hosts_file_override",
    ] {
        assert!(SettingsAdapter::new(SettingsNamespace::Global, key, "off", Some("auto")).is_err());
    }
}
