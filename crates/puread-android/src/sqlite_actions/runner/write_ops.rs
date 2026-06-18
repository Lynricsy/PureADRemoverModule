use std::fs::{File, Permissions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use puread_core::restore_ledger::OriginalFileType;

use crate::secure_fs::{
    ensure_parent_no_symlink, open_create_new_read_write_no_follow, open_read_no_follow,
    open_read_write_no_follow,
};
use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::metadata::{SqliteTargetMetadata, identity_changed};
use crate::sqlite_actions::minimal::validate_sqlite_image;

pub(super) fn open_write_no_follow(
    path: &Path,
    create_new: bool,
) -> Result<File, SqliteActionError> {
    if create_new {
        return open_create_new_read_write_no_follow(path).map_err(io_error(path));
    }
    open_read_write_no_follow(path).map_err(io_error(path))
}

pub(super) fn open_existing_for_delete(path: &Path) -> Result<File, SqliteActionError> {
    open_existing_read_no_follow(path)
}

pub(super) fn open_existing_read_no_follow(path: &Path) -> Result<File, SqliteActionError> {
    open_read_no_follow(path).map_err(io_error(path))
}

#[cfg(unix)]
pub(super) fn set_mode(file: &File, path: &Path, mode: u32) -> Result<(), SqliteActionError> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(Permissions::from_mode(mode))
        .map_err(|source| SqliteActionError::Io {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
pub(super) fn set_mode(file: &File, path: &Path, _mode: u32) -> Result<(), SqliteActionError> {
    let mut permissions = file
        .metadata()
        .map_err(|source| SqliteActionError::Io {
            path: path.to_path_buf(),
            source,
        })?
        .permissions();
    permissions.set_readonly(true);
    file.set_permissions(permissions)
        .map_err(|source| SqliteActionError::Io {
            path: path.to_path_buf(),
            source,
        })
}

pub(super) fn validate_written_file(path: &Path, file: &mut File) -> Result<(), SqliteActionError> {
    file.flush().map_err(|source| SqliteActionError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    file.seek(SeekFrom::Start(0))
        .map_err(|source| SqliteActionError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    let mut image = Vec::new();
    file.read_to_end(&mut image)
        .map_err(|source| SqliteActionError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    validate_sqlite_image(path, &image)
}

pub(super) fn guard_file_still_addressed(
    path: &Path,
    file: &File,
) -> Result<(), SqliteActionError> {
    let open_metadata = SqliteTargetMetadata::collect_open_file(path, file)?;
    ensure_parent_no_symlink(path).map_err(io_error(path))?;
    let path_metadata = SqliteTargetMetadata::collect(path)?;
    if path_metadata.file_type != open_metadata.file_type {
        return invalid_changed(path);
    }
    if identity_changed(open_metadata.identity, path_metadata.identity) {
        return invalid_changed(path);
    }
    Ok(())
}

pub(super) fn guard_file_matches_original(
    path: &Path,
    file: &File,
    original: &SqliteTargetMetadata,
) -> Result<(), SqliteActionError> {
    if original.file_type == OriginalFileType::Missing {
        return Ok(());
    }
    let open_metadata = SqliteTargetMetadata::collect_open_file(path, file)?;
    if open_metadata.file_type != original.file_type {
        return invalid_changed(path);
    }
    if identity_changed(original.identity, open_metadata.identity) {
        return invalid_changed(path);
    }
    Ok(())
}

fn invalid_changed(path: &Path) -> Result<(), SqliteActionError> {
    Err(SqliteActionError::InvalidTarget {
        path: path.to_path_buf(),
        reason: "sqlite target changed during mutation",
    })
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> SqliteActionError + '_ {
    |source| SqliteActionError::Io {
        path: path.to_path_buf(),
        source,
    }
}
