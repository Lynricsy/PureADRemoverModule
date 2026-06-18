use std::fs;
use std::path::{Path, PathBuf};

use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::target::host::reject_symlink;
use crate::sqlite_actions::target::path_kind::{has_bad_component, invalid};

pub(super) fn validate_host_database_mapping(
    android_path: &Path,
    host_path: &Path,
    filesystem_root: &Path,
) -> Result<(), SqliteActionError> {
    if !host_path.is_absolute() || !filesystem_root.is_absolute() {
        return invalid(android_path, "host sqlite paths must be absolute");
    }
    if has_bad_component(host_path) || has_bad_component(filesystem_root) {
        return invalid(android_path, "host sqlite path must not contain traversal");
    }
    if !host_path.starts_with(filesystem_root) {
        return invalid(android_path, "host sqlite path escapes filesystem root");
    }
    if host_path != expected_host_path(android_path, filesystem_root) {
        return invalid(android_path, "host sqlite path does not match android path");
    }
    reject_symlink(host_path).map_err(|_error| SqliteActionError::InvalidTarget {
        path: android_path.to_path_buf(),
        reason: "sqlite symlink targets are rejected",
    })?;
    let root = canonicalize(filesystem_root)?;
    let existing = existing_path_for_canonical_check(host_path, filesystem_root);
    let resolved = canonicalize(existing)?;
    if resolved.starts_with(root) {
        Ok(())
    } else {
        invalid(
            android_path,
            "canonical host sqlite path escapes filesystem root",
        )
    }
}

fn expected_host_path(android_path: &Path, filesystem_root: &Path) -> PathBuf {
    let relative = android_path
        .strip_prefix("/")
        .map_or(android_path, |path| path);
    filesystem_root.join(relative)
}

fn canonicalize(path: &Path) -> Result<PathBuf, SqliteActionError> {
    fs::canonicalize(path).map_err(|source| SqliteActionError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn existing_path_for_canonical_check<'a>(
    host_path: &'a Path,
    filesystem_root: &'a Path,
) -> &'a Path {
    if host_path.exists() {
        return host_path;
    }
    host_path.parent().map_or(filesystem_root, |parent| parent)
}
