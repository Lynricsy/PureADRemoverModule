#![doc = "`chattr +i` 强力 profile 适配层行为测试。"]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;

use puread_android::chattr::{
    CommandInvocation, CommandOutput, CommandRunner, ImmutableError, ImmutableOutcome,
    ImmutableProfile, ImmutableRequest, apply_immutable,
};
use puread_core::model::ProfileKind;

#[derive(Debug, Clone)]
struct ScriptedOutput {
    program: String,
    args: Vec<String>,
    result: Result<CommandOutput, ImmutableError>,
}

#[derive(Debug, Default)]
struct ScriptedRunner {
    expected: RefCell<VecDeque<ScriptedOutput>>,
    calls: RefCell<Vec<CommandInvocation>>,
}

impl ScriptedRunner {
    fn with_outputs(outputs: Vec<ScriptedOutput>) -> Self {
        Self {
            expected: RefCell::new(VecDeque::from(outputs)),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<CommandInvocation> {
        self.calls.borrow().clone()
    }
}

impl CommandRunner for ScriptedRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, ImmutableError> {
        self.calls.borrow_mut().push(invocation.clone());
        let Some(output) = self.expected.borrow_mut().pop_front() else {
            return Err(ImmutableError::UnscriptedCommand {
                program: invocation.program().to_owned(),
                args: invocation.args().to_vec(),
            });
        };
        assert_eq!(output.program, invocation.program());
        assert_eq!(output.args, invocation.args());
        output.result
    }
}

fn ok(stdout: &str) -> CommandOutput {
    CommandOutput::success(stdout, "")
}

fn fail(program: &str, args: &[&str], stderr: &str) -> CommandOutput {
    CommandOutput::failure(1, stderr, format!("{program} {}", args.join(" ")))
}

fn output_ok(program: &str, args: &[&str], result: CommandOutput) -> ScriptedOutput {
    output(program, args, Ok(result))
}

fn output_err(program: &str, args: &[&str], error: ImmutableError) -> ScriptedOutput {
    output(program, args, Err(error))
}

fn output(
    program: &str,
    args: &[&str],
    result: Result<CommandOutput, ImmutableError>,
) -> ScriptedOutput {
    ScriptedOutput {
        program: program.to_owned(),
        args: args.iter().map(ToString::to_string).collect(),
        result,
    }
}

#[test]
fn chattr_skips_without_command_execution_when_profile_is_not_strong() {
    // Given: a default profile attempts to request immutable mode.
    let runner = ScriptedRunner::default();
    let request = ImmutableRequest::new(
        Path::new("/data/user/0/com.example/cache/ad"),
        ImmutableProfile::from(ProfileKind::Conservative),
    );

    // When: immutable application is evaluated.
    let outcome = apply_immutable(&runner, &request);

    // Then: the plan is observable and no command is executed.
    assert!(matches!(outcome, ImmutableOutcome::Skipped { .. }));
    assert!(runner.calls().is_empty());
}

#[test]
fn chattr_records_original_attrs_and_applies_immutable_when_profile_is_strong() {
    // Given: both commands exist, original attributes can be read, and chattr succeeds.
    let runner = ScriptedRunner::with_outputs(vec![
        output_ok("/system/bin/lsattr", &[], ok("usage")),
        output_ok("/system/bin/chattr", &[], ok("usage")),
        output_ok(
            "/system/bin/lsattr",
            &["/data/user/0/com.example/cache/ad"],
            ok("--------------e------- /data/user/0/com.example/cache/ad\n"),
        ),
        output_ok(
            "/system/bin/chattr",
            &["+i", "/data/user/0/com.example/cache/ad"],
            ok(""),
        ),
    ]);
    let request = ImmutableRequest::new(
        Path::new("/data/user/0/com.example/cache/ad"),
        ImmutableProfile::Strong,
    );

    // When: immutable mode is applied.
    let outcome = apply_immutable(&runner, &request);

    // Then: the original attributes and executed command are observable.
    let ImmutableOutcome::Applied { original_attrs, .. } = outcome else {
        panic!("expected applied outcome, got {outcome:?}");
    };
    assert_eq!(original_attrs.as_deref(), Some("--------------e-------"));
    let calls = runner.calls();
    assert_eq!(calls.len(), 4);
    assert!(matches!(
        calls.get(3),
        Some(call)
            if call.program() == "/system/bin/chattr"
                && call.args()
                    == ["+i".to_owned(), "/data/user/0/com.example/cache/ad".to_owned()]
    ));
}

#[test]
fn degrades_without_panic_when_chattr_command_is_missing() {
    // Given: lsattr exists but chattr is missing on the Android image.
    let runner = ScriptedRunner::with_outputs(vec![
        output_ok("/system/bin/lsattr", &[], ok("usage")),
        output_err(
            "/system/bin/chattr",
            &[],
            ImmutableError::CommandUnavailable {
                program: "/system/bin/chattr".to_owned(),
                detail: "not found".to_owned(),
            },
        ),
        output_err(
            "/system/xbin/chattr",
            &[],
            ImmutableError::CommandUnavailable {
                program: "/system/xbin/chattr".to_owned(),
                detail: "not found".to_owned(),
            },
        ),
        output_err(
            "/vendor/bin/chattr",
            &[],
            ImmutableError::CommandUnavailable {
                program: "/vendor/bin/chattr".to_owned(),
                detail: "not found".to_owned(),
            },
        ),
    ]);
    let request = ImmutableRequest::new(
        Path::new("/data/user/0/com.example/cache/ad"),
        ImmutableProfile::Strong,
    );

    // When: immutable mode is applied.
    let outcome = apply_immutable(&runner, &request);

    // Then: the caller gets a degraded observable result and no chattr mutation runs.
    assert!(matches!(outcome, ImmutableOutcome::Degraded { .. }));
    assert_eq!(runner.calls().len(), 4);
}

#[test]
fn degrades_with_original_attrs_when_chattr_fails() {
    // Given: original attributes are visible but chattr returns a non-zero status.
    let runner = ScriptedRunner::with_outputs(vec![
        output_ok("/system/bin/lsattr", &[], ok("usage")),
        output_ok("/system/bin/chattr", &[], ok("usage")),
        output_ok(
            "/system/bin/lsattr",
            &["/data/user/0/com.example/cache/ad"],
            ok("----i---------e------- /data/user/0/com.example/cache/ad\n"),
        ),
        output_ok(
            "/system/bin/chattr",
            &["+i", "/data/user/0/com.example/cache/ad"],
            fail(
                "/system/bin/chattr",
                &["+i", "/data/user/0/com.example/cache/ad"],
                "operation not permitted",
            ),
        ),
    ]);
    let request = ImmutableRequest::new(
        Path::new("/data/user/0/com.example/cache/ad"),
        ImmutableProfile::Strong,
    );

    // When: immutable mode is applied.
    let outcome = apply_immutable(&runner, &request);

    // Then: the failure is downgraded and the original immutable state remains observable.
    let ImmutableOutcome::Degraded { original_attrs, .. } = outcome else {
        panic!("expected degraded outcome, got {outcome:?}");
    };
    assert_eq!(original_attrs.as_deref(), Some("----i---------e-------"));
}

#[test]
fn degrades_when_chattr_claims_success_but_output_reports_failure() {
    // Given: a toybox/busybox variant exits zero but prints a permission error.
    let runner = ScriptedRunner::with_outputs(vec![
        output_ok("/system/bin/lsattr", &[], ok("usage")),
        output_ok("/system/bin/chattr", &[], ok("usage")),
        output_ok(
            "/system/bin/lsattr",
            &["/data/user/0/com.example/cache/ad"],
            ok("--------------e------- /data/user/0/com.example/cache/ad\n"),
        ),
        output_ok(
            "/system/bin/chattr",
            &["+i", "/data/user/0/com.example/cache/ad"],
            CommandOutput::success("", "operation not permitted"),
        ),
    ]);
    let request = ImmutableRequest::new(
        Path::new("/data/user/0/com.example/cache/ad"),
        ImmutableProfile::Strong,
    );

    // When: immutable mode is applied.
    let outcome = apply_immutable(&runner, &request);

    // Then: misleading success output is still observable as a degraded result.
    assert!(matches!(outcome, ImmutableOutcome::Degraded { .. }));
}
