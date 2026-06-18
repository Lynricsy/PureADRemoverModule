use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::secure_fs::{create_dir_all_no_symlink, create_new_no_follow};
use crate::sqlite_actions::error::SqliteActionError;

pub(super) fn backup_path_for(
    backup_dir: &Path,
    path: &Path,
    rule_id: &str,
) -> Result<PathBuf, SqliteActionError> {
    create_dir_all_no_symlink(backup_dir).map_err(|source| SqliteActionError::Io {
        path: backup_dir.to_path_buf(),
        source,
    })?;
    Ok(backup_dir.join(backup_name(rule_id, path)))
}

pub(super) fn copy_open_file_to_backup(
    source: &mut File,
    source_path: &Path,
    backup_path: &Path,
) -> Result<bool, SqliteActionError> {
    if backup_path_is_occupied(backup_path)? {
        return Err(SqliteActionError::InvalidTarget {
            path: backup_path.to_path_buf(),
            reason: "sqlite backup already exists before backup",
        });
    }
    source
        .seek(SeekFrom::Start(0))
        .map_err(|source| SqliteActionError::Io {
            path: source_path.to_path_buf(),
            source,
        })?;
    let mut backup = create_new_no_follow(backup_path).map_err(|source| SqliteActionError::Io {
        path: backup_path.to_path_buf(),
        source,
    })?;
    let mut buffer = Vec::new();
    source
        .read_to_end(&mut buffer)
        .map_err(|source| SqliteActionError::Io {
            path: source_path.to_path_buf(),
            source,
        })?;
    backup
        .write_all(&buffer)
        .and_then(|()| backup.flush())
        .map_err(|source| SqliteActionError::Io {
            path: backup_path.to_path_buf(),
            source,
        })?;
    Ok(true)
}

fn backup_path_is_occupied(path: &Path) -> Result<bool, SqliteActionError> {
    match std::fs::symlink_metadata(path) {
        Ok(_metadata) => Ok(true),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(SqliteActionError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn backup_name(rule_id: &str, path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!(
        "{}-{:016x}.sqlite",
        sanitize_rule_id(rule_id),
        hasher.finish()
    )
}

fn sanitize_rule_id(rule_id: &str) -> String {
    rule_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
