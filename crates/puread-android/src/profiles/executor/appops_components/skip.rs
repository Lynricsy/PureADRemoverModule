use crate::command_runner::CommandError;
use crate::profiles::error::ProfileError;

pub(super) fn package_probe_missing(error: &ProfileError) -> bool {
    match error {
        ProfileError::Command(CommandError::CommandFailed { stderr, stdout, .. }) => {
            command_text_mentions_missing_package(stderr)
                || command_text_mentions_missing_package(stdout)
        }
        _ => false,
    }
}

fn command_text_mentions_missing_package(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("unknown package")
        || value.contains("can't find package")
        || value.contains("not installed")
        || value.contains("package not found")
}
