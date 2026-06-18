mod command;
mod error;
mod outcome;
mod profile;

use std::path::Path;

pub use command::{CommandInvocation, CommandOutput, CommandRunner, RealCommandRunner};
pub use error::ImmutableError;
pub use outcome::{ImmutableOutcome, ObservedCommand};
pub use profile::ImmutableProfile;

const LSATTR_CANDIDATES: [&str; 3] = [
    "/system/bin/lsattr",
    "/system/xbin/lsattr",
    "/vendor/bin/lsattr",
];
const CHATTR_CANDIDATES: [&str; 3] = [
    "/system/bin/chattr",
    "/system/xbin/chattr",
    "/vendor/bin/chattr",
];
const FAILURE_MARKERS: [&str; 4] = [
    "not permitted",
    "permission denied",
    "operation failed",
    "error",
];

/// 单次 immutable 应用请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImmutableRequest {
    target: String,
    profile: ImmutableProfile,
}

impl ImmutableRequest {
    /// 构造已限定目标路径和 profile 的 immutable 请求。
    #[must_use]
    pub fn new(target: &Path, profile: ImmutableProfile) -> Self {
        Self {
            target: target.display().to_string(),
            profile,
        }
    }

    /// 返回目标路径文本。
    #[must_use]
    pub const fn target(&self) -> &str {
        self.target.as_str()
    }

    /// 返回 immutable profile。
    #[must_use]
    pub const fn profile(&self) -> ImmutableProfile {
        self.profile
    }
}

/// 在强力 profile 下尝试执行 `chattr +i`。
#[must_use]
pub fn apply_immutable<R>(runner: &R, request: &ImmutableRequest) -> ImmutableOutcome
where
    R: CommandRunner,
{
    if !request.profile().is_strong() {
        return ImmutableOutcome::Skipped {
            target: request.target().to_owned(),
            reason: "immutable mode requires strong profile".to_owned(),
        };
    }

    let lsattr = match resolve_command(runner, &LSATTR_CANDIDATES) {
        Ok(command) => command,
        Err(error) => return degraded(request, None, "lsattr command unavailable", error),
    };
    let chattr = match resolve_command(runner, &CHATTR_CANDIDATES) {
        Ok(command) => command,
        Err(error) => return degraded(request, None, "chattr command unavailable", error),
    };

    let original_attrs = match read_attrs(runner, lsattr, request.target()) {
        Ok(attrs) => Some(attrs),
        Err(error) => return degraded(request, None, "lsattr failed before mutation", error),
    };

    match run_chattr(runner, chattr, request.target()) {
        Ok(command) => ImmutableOutcome::Applied {
            target: request.target().to_owned(),
            original_attrs,
            command,
        },
        Err(error) => degraded(request, original_attrs, "chattr failed", error),
    }
}

fn resolve_command<'a, R>(
    runner: &R,
    candidates: &'a [&'static str],
) -> Result<&'a str, ImmutableError>
where
    R: CommandRunner,
{
    for candidate in candidates {
        let invocation = CommandInvocation::new(candidate, std::iter::empty::<&str>());
        match runner.run(&invocation) {
            Ok(_output) => return Ok(candidate),
            Err(_error) => {}
        }
    }
    Err(ImmutableError::CommandUnavailable {
        program: candidates
            .first()
            .map_or("chattr-tools", |candidate| candidate)
            .to_string(),
        detail: "no trusted absolute command path was executable".to_owned(),
    })
}

fn read_attrs<R>(runner: &R, program: &str, target: &str) -> Result<String, ImmutableError>
where
    R: CommandRunner,
{
    let invocation = CommandInvocation::new(program, [target]);
    let output = runner.run(&invocation)?;
    if !output.is_success_without_failure_marker() {
        return Err(ImmutableError::CommandFailed {
            program: program.to_owned(),
            args: invocation.args().to_vec(),
            status: output.status(),
            stdout: output.stdout().to_owned(),
            stderr: output.stderr().to_owned(),
        });
    }
    parse_lsattr(output.stdout())
}

fn run_chattr<R>(runner: &R, program: &str, target: &str) -> Result<ObservedCommand, ImmutableError>
where
    R: CommandRunner,
{
    let invocation = CommandInvocation::new(program, ["+i", target]);
    let output = runner.run(&invocation)?;
    if output.is_success_without_failure_marker() {
        return Ok(ObservedCommand::from_invocation(
            &invocation,
            output.status(),
        ));
    }
    Err(ImmutableError::CommandFailed {
        program: program.to_owned(),
        args: invocation.args().to_vec(),
        status: output.status(),
        stdout: output.stdout().to_owned(),
        stderr: output.stderr().to_owned(),
    })
}

fn degraded(
    request: &ImmutableRequest,
    original_attrs: Option<String>,
    reason: &'static str,
    error: ImmutableError,
) -> ImmutableOutcome {
    ImmutableOutcome::Degraded {
        target: request.target().to_owned(),
        original_attrs,
        reason,
        error,
    }
}

fn parse_lsattr(stdout: &str) -> Result<String, ImmutableError> {
    let Some(line) = stdout.lines().find(|line| !line.trim().is_empty()) else {
        return Err(ImmutableError::MalformedLsattr {
            stdout: stdout.to_owned(),
        });
    };
    let Some((attrs, _path)) = line.split_once(char::is_whitespace) else {
        return Err(ImmutableError::MalformedLsattr {
            stdout: stdout.to_owned(),
        });
    };
    if attrs.is_empty() {
        return Err(ImmutableError::MalformedLsattr {
            stdout: stdout.to_owned(),
        });
    }
    Ok(attrs.to_owned())
}

pub(crate) fn contains_failure_marker(stdout: &str, stderr: &str) -> bool {
    let combined = format!(
        "{}\n{}",
        stdout.to_ascii_lowercase(),
        stderr.to_ascii_lowercase()
    );
    FAILURE_MARKERS
        .iter()
        .any(|marker| combined.contains(marker))
}
