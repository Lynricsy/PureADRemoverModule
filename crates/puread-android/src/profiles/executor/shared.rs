use crate::command_runner::{AndroidCommandRunner, CommandError, CommandInvocation, CommandOutput};
use crate::profiles::error::ProfileError;
use crate::profiles::record::ProfileRecord;

pub(super) fn parse_record(record: &str) -> Result<ProfileRecord, ProfileError> {
    serde_json::from_str(record).map_err(ProfileError::json)
}

pub(super) fn run_required<R>(
    runner: &R,
    invocation: &CommandInvocation,
) -> Result<CommandOutput, ProfileError>
where
    R: AndroidCommandRunner,
{
    match runner.run(invocation) {
        Ok(output) if output.status() == 0 => Ok(output),
        Ok(output) => Err(ProfileError::Command(CommandError::CommandFailed {
            invocation: invocation.clone(),
            status: output.status(),
            stdout: output.stdout().to_owned(),
            stderr: output.stderr().to_owned(),
        })),
        Err(error) => Err(ProfileError::Runner {
            detail: error.to_string(),
        }),
    }
}
