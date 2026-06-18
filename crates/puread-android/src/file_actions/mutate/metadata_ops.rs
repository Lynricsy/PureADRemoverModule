use std::path::Path;

use puread_core::model::RiskLevel;
use puread_core::restore_ledger::OriginalFileType;

use crate::command_runner::{
    AndroidCommandRunner, CommandInvocation, CommandOutput, CommandRunnerError,
    RealAndroidCommandRunner,
};
use crate::file_actions::error::FileActionError;
use crate::file_actions::metadata::{MetadataChange, MetadataOperation};
use crate::file_actions::mutate::guards::{
    allowed_after_missing, fd_path_for_metadata_operation, guard_open_file_matches_snapshot,
    guard_path_for_snapshot, open_existing_no_follow, run_before_file_helper_guard_for_tests,
};
use crate::file_actions::plan::FileActionPlan;
use crate::file_actions::snapshot::TargetSnapshot;

const CHCON_CANDIDATES: [&str; 3] = [
    "/system/bin/chcon",
    "/system/xbin/chcon",
    "/vendor/bin/chcon",
];

pub(in crate::file_actions) fn apply_metadata_changes(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
) -> Result<Vec<MetadataOperation>, FileActionError> {
    let mut operations = Vec::new();
    for change in plan.metadata_changes() {
        apply_one_metadata_change(plan, snapshot, change, &mut operations)?;
    }
    Ok(operations)
}

fn apply_one_metadata_change(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
    change: &MetadataChange,
    operations: &mut Vec<MetadataOperation>,
) -> Result<(), FileActionError> {
    match change {
        MetadataChange::SetOwner { uid, gid } => {
            set_owner(
                plan.target().host_path(),
                snapshot,
                allowed_after_missing(plan.action()),
                *uid,
                *gid,
            )?;
            operations.push(MetadataOperation::SetOwner {
                uid: *uid,
                gid: *gid,
            });
        }
        MetadataChange::SetSelinuxContext { context } => {
            set_selinux_context(plan, snapshot, context, operations)?;
        }
        MetadataChange::RestoreSelinuxContext => restore_context(plan, snapshot, operations)?,
    }
    Ok(())
}

fn set_selinux_context(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
    context: &str,
    operations: &mut Vec<MetadataOperation>,
) -> Result<(), FileActionError> {
    if plan.risk_level() == RiskLevel::High && snapshot.selinux_context.is_none() {
        operations.push(MetadataOperation::SkippedChconUnknownContext);
        return Ok(());
    }
    chcon(
        plan.target().host_path(),
        snapshot,
        allowed_after_missing(plan.action()),
        context,
    )?;
    operations.push(MetadataOperation::Chcon {
        context: context.to_owned(),
    });
    Ok(())
}

fn restore_context(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
    operations: &mut Vec<MetadataOperation>,
) -> Result<(), FileActionError> {
    if let Some(context) = &snapshot.selinux_context {
        chcon(
            plan.target().host_path(),
            snapshot,
            allowed_after_missing(plan.action()),
            context,
        )?;
        operations.push(MetadataOperation::Chcon {
            context: context.clone(),
        });
    } else {
        operations.push(MetadataOperation::SkippedChconUnknownContext);
    }
    Ok(())
}

#[cfg(unix)]
fn set_owner(
    path: &Path,
    snapshot: &TargetSnapshot,
    allowed_after_missing: &[OriginalFileType],
    uid: u32,
    gid: u32,
) -> Result<(), FileActionError> {
    run_before_file_helper_guard_for_tests(path);
    guard_path_for_snapshot(path, snapshot, allowed_after_missing)?;
    let file = open_existing_no_follow(path)?;
    guard_open_file_matches_snapshot(path, snapshot, &file)?;
    let fd_path = fd_path_for_metadata_operation(&file, path)?;
    std::os::unix::fs::chown(&fd_path, Some(uid), Some(gid))
        .map_err(|source| FileActionError::io(&fd_path, source))
}

#[cfg(not(unix))]
fn set_owner(
    path: &Path,
    _snapshot: &TargetSnapshot,
    _allowed_after_missing: &[OriginalFileType],
    _uid: u32,
    _gid: u32,
) -> Result<(), FileActionError> {
    Err(FileActionError::rejected_target(
        path,
        "chown requires unix filesystem",
    ))
}

fn chcon(
    path: &Path,
    snapshot: &TargetSnapshot,
    allowed_after_missing: &[OriginalFileType],
    context: &str,
) -> Result<(), FileActionError> {
    run_before_file_helper_guard_for_tests(path);
    guard_path_for_snapshot(path, snapshot, allowed_after_missing)?;
    let file = open_existing_no_follow(path)?;
    guard_open_file_matches_snapshot(path, snapshot, &file)?;
    let fd_path = fd_path_for_metadata_operation(&file, path)?;
    run_trusted_chcon(&CHCON_CANDIDATES, context, &fd_path)
}

fn run_trusted_chcon(
    candidates: &[&str],
    context: &str,
    fd_path: &Path,
) -> Result<(), FileActionError> {
    run_trusted_chcon_with(&RealAndroidCommandRunner, candidates, context, fd_path)
}

fn run_trusted_chcon_with<R>(
    runner: &R,
    candidates: &[&str],
    context: &str,
    fd_path: &Path,
) -> Result<(), FileActionError>
where
    R: AndroidCommandRunner,
{
    let fd_path_text = fd_path.to_string_lossy();
    for candidate in candidates {
        let invocation = CommandInvocation::new(candidate, [context, fd_path_text.as_ref()]);
        let output = match runner.run(&invocation) {
            Ok(output) => output,
            Err(CommandRunnerError::NotFound { .. }) => continue,
            Err(CommandRunnerError::Unavailable { detail }) => {
                return Err(FileActionError::Metadata {
                    operation: "chcon",
                    path: fd_path.to_path_buf(),
                    detail,
                });
            }
        };
        if output.status() == 0 {
            return Ok(());
        }
        return Err(FileActionError::Metadata {
            operation: "chcon",
            path: fd_path.to_path_buf(),
            detail: command_failure_detail(&output),
        });
    }
    Err(FileActionError::Metadata {
        operation: "chcon",
        path: fd_path.to_path_buf(),
        detail: "trusted chcon command unavailable".to_owned(),
    })
}

fn command_failure_detail(output: &CommandOutput) -> String {
    if output.stderr().is_empty() {
        return output.stdout().to_owned();
    }
    output.stderr().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn chcon_executes_first_trusted_absolute_candidate() {
        use std::os::unix::fs::PermissionsExt;

        let dir =
            std::env::temp_dir().join(format!("puread-chcon-candidate-{}", std::process::id()));
        let script = dir.join("trusted-chcon");
        let capture = dir.join("capture");
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test chcon dir should be created");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$1\" > {}\nprintf '%s\\n' \"$2\" >> {}\n",
                capture.display(),
                capture.display()
            ),
        )
        .expect("test chcon script should be written");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("test chcon script should be executable");

        run_trusted_chcon_with(
            &RealAndroidCommandRunner,
            &[script.to_str().expect("test path should be utf-8")],
            "u:object_r:cache_file:s0",
            Path::new("/proc/self/fd/0"),
        )
        .expect("absolute trusted chcon candidate should run");

        assert_eq!(
            std::fs::read_to_string(&capture).expect("test chcon output should be captured"),
            "u:object_r:cache_file:s0\n/proc/self/fd/0\n"
        );
        std::fs::remove_dir_all(&dir).expect("test chcon dir should be removed");
    }

    #[test]
    fn chcon_reports_unavailable_after_all_trusted_candidates_are_missing() {
        // Given: only absolute paths that do not exist on the host are considered.
        let candidates = ["/puread-test-missing/chcon"];
        let path = Path::new("/proc/self/fd/0");

        // When: every trusted candidate returns NotFound.
        let error = run_trusted_chcon(&candidates, "u:object_r:cache_file:s0", path)
            .expect_err("missing trusted chcon should fail");

        // Then: the error reports trusted chcon unavailability instead of PATH fallback.
        assert!(matches!(
            error,
            FileActionError::Metadata {
                operation: "chcon",
                detail,
                ..
            } if detail == "trusted chcon command unavailable"
        ));
    }
}
