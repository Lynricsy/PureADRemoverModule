use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use puread_core::restore_ledger::OriginalFileType;

use crate::file_actions::error::FileActionError;
use crate::file_actions::plan::FileActionPlan;
use crate::file_actions::request::FileActionKind;
use crate::file_actions::snapshot::TargetSnapshot;
use crate::secure_fs::{
    create_dir_all_no_symlink, create_new_no_follow, ensure_parent_no_symlink, open_read_no_follow,
};

pub(super) fn backup_original(
    backup_dir: &Path,
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
) -> Result<Option<PathBuf>, FileActionError> {
    if snapshot.original_type == OriginalFileType::Missing {
        return Ok(None);
    }
    create_dir_all_no_symlink(backup_dir)
        .map_err(|source| FileActionError::io(backup_dir, source))?;
    let backup_path = backup_dir.join(backup_name(plan));
    if backup_path_is_occupied(&backup_path)? {
        return Err(FileActionError::rejected_target(
            backup_path,
            "backup path already exists before backup",
        ));
    }
    match (snapshot.original_type, plan.action()) {
        (OriginalFileType::File | OriginalFileType::Directory, FileActionKind::Delete)
        | (OriginalFileType::Directory, FileActionKind::EmptyDir) => {}
        (OriginalFileType::File, _) => {
            copy_bound_file(plan.target().host_path(), &backup_path, snapshot)?;
        }
        (OriginalFileType::Directory, _) => {
            return Err(FileActionError::rejected_target(
                plan.target().android_path(),
                "directory backup requires move-based mutation",
            ));
        }
        (OriginalFileType::Missing, _) => return Ok(None),
        (OriginalFileType::Symlink | OriginalFileType::Other, _) => {
            return Err(FileActionError::rejected_target(
                plan.target().android_path(),
                "unsupported file type",
            ));
        }
    }
    Ok(Some(backup_path))
}

fn backup_path_is_occupied(path: &Path) -> Result<bool, FileActionError> {
    match std::fs::symlink_metadata(path) {
        Ok(_metadata) => Ok(true),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(FileActionError::io(path, source)),
    }
}

fn copy_bound_file(
    source: &Path,
    target: &Path,
    snapshot: &TargetSnapshot,
) -> Result<(), FileActionError> {
    ensure_parent(target)?;
    let mut source_file =
        open_read_no_follow(source).map_err(|error| FileActionError::io(source, error))?;
    guard_backup_file_matches_snapshot(source, snapshot, &source_file)?;
    let mut target_file =
        create_new_no_follow(target).map_err(|error| FileActionError::io(target, error))?;
    std::io::copy(&mut source_file, &mut target_file)
        .and_then(|_bytes| target_file.flush())
        .map_err(|error| FileActionError::io(target, error))
}

fn guard_backup_file_matches_snapshot(
    path: &Path,
    snapshot: &TargetSnapshot,
    file: &File,
) -> Result<(), FileActionError> {
    let current = TargetSnapshot::collect_open_file(path, file)?;
    if current.original_type != snapshot.original_type
        || snapshot.identity_changed_from_metadata(
            &file
                .metadata()
                .map_err(|error| FileActionError::io(path, error))?,
        )
    {
        return Err(FileActionError::rejected_target(
            path,
            "target changed before backup",
        ));
    }
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), FileActionError> {
    ensure_parent_no_symlink(path).map_err(|source| FileActionError::io(path, source))
}

fn backup_name(plan: &FileActionPlan) -> String {
    format!(
        "{}-{}-{}.bak",
        plan.rule_id().as_str(),
        plan.action().as_str(),
        sanitize_path(plan.target().android_path())
    )
}

fn sanitize_path(path: &Path) -> String {
    path.display()
        .to_string()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
