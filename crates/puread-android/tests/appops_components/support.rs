#![allow(
    clippy::redundant_pub_crate,
    reason = "integration test support helpers are imported from their parent test module"
)]

use std::cell::RefCell;
use std::collections::VecDeque;

use puread_android::command_runner::{
    AndroidCommandRunner, CommandInvocation, CommandOutput, CommandRunnerError,
};
use puread_android::profiles::{
    AndroidProfileExecutor, AppOpProfileRule, ComponentProfileRule, PmHidePolicy, ProfileError,
    ProfileLedgerSink,
};

#[derive(Debug, Default)]
pub(super) struct ScriptedRunner {
    outputs: RefCell<VecDeque<Result<CommandOutput, CommandRunnerError>>>,
    calls: RefCell<Vec<CommandInvocation>>,
}

impl ScriptedRunner {
    pub(super) fn with_outputs(outputs: Vec<CommandOutput>) -> Self {
        Self {
            outputs: RefCell::new(VecDeque::from(
                outputs.into_iter().map(Ok).collect::<Vec<_>>(),
            )),
            calls: RefCell::new(Vec::new()),
        }
    }

    pub(super) fn with_script(outputs: Vec<Result<CommandOutput, CommandRunnerError>>) -> Self {
        Self {
            outputs: RefCell::new(VecDeque::from(outputs)),
            calls: RefCell::new(Vec::new()),
        }
    }

    pub(super) fn call_lines(&self) -> Vec<String> {
        self.calls
            .borrow()
            .iter()
            .map(CommandInvocation::argv)
            .map(|argv| argv.join(" "))
            .collect()
    }
}

impl AndroidCommandRunner for ScriptedRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
        self.calls.borrow_mut().push(invocation.clone());
        self.outputs.borrow_mut().pop_front().unwrap_or_else(|| {
            Err(CommandRunnerError::Unavailable {
                detail: "unscripted fake command".to_owned(),
            })
        })
    }
}

#[derive(Debug, Default)]
pub(super) struct MemoryLedger {
    records: RefCell<Vec<String>>,
    fail: bool,
    fail_after_successes: Option<usize>,
}

impl MemoryLedger {
    pub(super) const fn failing() -> Self {
        Self {
            records: RefCell::new(Vec::new()),
            fail: true,
            fail_after_successes: None,
        }
    }

    pub(super) const fn failing_after_successes(count: usize) -> Self {
        Self {
            records: RefCell::new(Vec::new()),
            fail: false,
            fail_after_successes: Some(count),
        }
    }

    pub(super) fn records(&self) -> Vec<String> {
        self.records.borrow().clone()
    }
}

impl ProfileLedgerSink for MemoryLedger {
    fn append(&self, record: String) -> Result<(), ProfileError> {
        if self.fail
            || self
                .fail_after_successes
                .is_some_and(|count| self.records.borrow().len() >= count)
        {
            return Err(ProfileError::Runner {
                detail: "ledger sink failed".to_owned(),
            });
        }
        self.records.borrow_mut().push(record);
        Ok(())
    }
}

pub(super) fn appops_ledger_sink_failure_does_not_mutate() -> Result<(), Box<dyn std::error::Error>>
{
    let runner = ScriptedRunner::with_outputs(vec![
        CommandOutput::success("package:/data/app/base.apk\n", ""),
        CommandOutput::success("No operations.\n", ""),
    ]);
    let ledger = MemoryLedger::failing();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = AppOpProfileRule::new(
        "luna-background-location",
        "com.luna.music",
        "MONITOR_LOCATION",
        "ignore",
    )?;

    let Err(error) = executor.apply_appop(&rule) else {
        return Err("expected AppOps ledger failure".into());
    };

    assert!(error.to_string().contains("ledger sink failed"));
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm path com.luna.music",
            "/system/bin/cmd appops get com.luna.music MONITOR_LOCATION",
        ]
    );
    assert!(ledger.records().is_empty());
    Ok(())
}

pub(super) fn component_ledger_sink_failure_does_not_mutate()
-> Result<(), Box<dyn std::error::Error>> {
    let runner = ScriptedRunner::with_outputs(vec![
        CommandOutput::success("package:/data/app/base.apk\n", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
    ]);
    let ledger = MemoryLedger::failing();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = ComponentProfileRule::new(
        "mi-market-reverse-ad",
        0,
        "com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
        PmHidePolicy::TryHide,
    )?;

    let Err(error) = executor.apply_component(&rule) else {
        return Err("expected component ledger failure".into());
    };

    assert!(error.to_string().contains("ledger sink failed"));
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm path com.xiaomi.market",
            "/system/bin/pm list packages -d com.xiaomi.market",
            "/system/bin/pm list packages --hidden com.xiaomi.market",
        ]
    );
    assert!(ledger.records().is_empty());
    Ok(())
}
