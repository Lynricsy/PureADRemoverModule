#![allow(
    unreachable_pub,
    reason = "integration test support is imported through a private parent module"
)]

use std::cell::RefCell;
use std::collections::VecDeque;

use puread_android::command_runner::{
    AndroidCommandRunner, CommandInvocation, CommandOutput, CommandRunnerError,
};

#[derive(Debug, Default)]
pub struct ScriptedRunner {
    outputs: RefCell<VecDeque<CommandOutput>>,
    calls: RefCell<Vec<CommandInvocation>>,
}

impl ScriptedRunner {
    pub fn with_outputs(outputs: Vec<CommandOutput>) -> Self {
        Self {
            outputs: RefCell::new(VecDeque::from(outputs)),
            calls: RefCell::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> Vec<Vec<String>> {
        self.calls
            .borrow()
            .iter()
            .map(CommandInvocation::argv)
            .collect()
    }

    pub fn call_lines(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .map(|argv| argv.join(" "))
            .collect()
    }
}

impl AndroidCommandRunner for ScriptedRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
        self.calls.borrow_mut().push(invocation.clone());
        self.outputs
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| CommandRunnerError::Unavailable {
                detail: "unscripted fake command".to_owned(),
            })
    }
}

pub fn ok(stdout: &str) -> CommandOutput {
    CommandOutput::success(stdout, "")
}

pub fn fail(stderr: &str) -> CommandOutput {
    CommandOutput::failure(7, "", stderr)
}

pub fn command_lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}
