use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use crate::error::CliError;
use crate::json::display_path;
use crate::profile_execute::profile_restore::report::ProfileRestoreAction;

const O_NOFOLLOW: i32 = 0o400_000;

#[derive(Debug, Clone)]
pub(super) struct ProfileLedgerEntry {
    raw: String,
    kind: String,
    restored: bool,
}

impl ProfileLedgerEntry {
    pub(super) const fn raw(&self) -> &str {
        self.raw.as_str()
    }

    pub(super) const fn kind(&self) -> &str {
        self.kind.as_str()
    }

    pub(super) const fn restored(&self) -> bool {
        self.restored
    }
}

pub(super) fn read_entries(
    module_root: &Path,
    path: &Path,
) -> Result<Vec<ProfileLedgerEntry>, CliError> {
    ensure_safe_read_path(module_root, path)?;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)
        .map_err(|source| CliError::Filesystem {
            path: display_path(path),
            source,
        });
    let file = match file {
        Ok(file) => file,
        Err(CliError::Filesystem { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => return Err(error),
    };
    BufReader::new(file)
        .lines()
        .filter_map(|line| transpose_non_empty_line(line, path))
        .collect()
}

fn ensure_safe_read_path(module_root: &Path, path: &Path) -> Result<(), CliError> {
    ensure_module_child_path(module_root, path)?;
    ensure_existing_parent_components(path)?;
    ensure_profile_ledger_leaf_is_not_symlink(path)
}

pub(super) fn rewrite_restored(
    path: &Path,
    entries: &[ProfileLedgerEntry],
    actions: &[ProfileRestoreAction],
) -> Result<(), CliError> {
    if actions.iter().any(ProfileRestoreAction::failed) {
        return Ok(());
    }
    let restored_indexes = restored_indexes(actions);
    let mut output = String::new();
    for (index, entry) in entries.iter().enumerate() {
        if restored_indexes.contains(&index) {
            output.push_str(mark_restored(entry.raw.as_str())?.as_str());
        } else {
            output.push_str(entry.raw.as_str());
        }
        output.push('\n');
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)
        .map_err(|source| CliError::Filesystem {
            path: display_path(path),
            source,
        })?;
    file.write_all(output.as_bytes())
        .map_err(|source| CliError::Filesystem {
            path: display_path(path),
            source,
        })
}

fn restored_indexes(actions: &[ProfileRestoreAction]) -> BTreeSet<usize> {
    actions
        .iter()
        .filter(|action| action.restored())
        .flat_map(ProfileRestoreAction::entry_indexes)
        .copied()
        .collect()
}

fn transpose_non_empty_line(
    line: std::io::Result<String>,
    path: &Path,
) -> Option<Result<ProfileLedgerEntry, CliError>> {
    match line {
        Ok(value) if value.trim().is_empty() => None,
        Ok(value) => Some(parse_entry(value, path)),
        Err(source) => Some(Err(CliError::Filesystem {
            path: display_path(path),
            source,
        })),
    }
}

fn parse_entry(raw: String, path: &Path) -> Result<ProfileLedgerEntry, CliError> {
    let value = parse_value(raw.as_str(), path)?;
    let kind = value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CliError::ProfileLedgerMissingKind {
            path: display_path(path),
        })?
        .to_owned();
    let restored = value
        .get("restore_status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status == "restored");
    Ok(ProfileLedgerEntry {
        raw,
        kind,
        restored,
    })
}

fn mark_restored(raw: &str) -> Result<String, CliError> {
    let path = Path::new("profile ledger record");
    let mut value = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(raw)
        .map_err(|source| CliError::ProfileLedgerJson {
            path: display_path(path),
            source,
        })?;
    value.insert(
        "restore_status".to_owned(),
        serde_json::Value::String("restored".to_owned()),
    );
    serde_json::to_string(&value).map_err(|source| CliError::ProfileLedgerJson {
        path: display_path(path),
        source,
    })
}

fn parse_value(raw: &str, path: &Path) -> Result<serde_json::Value, CliError> {
    serde_json::from_str(raw).map_err(|source| CliError::ProfileLedgerJson {
        path: display_path(path),
        source,
    })
}

fn ensure_module_child_path(module_root: &Path, path: &Path) -> Result<(), CliError> {
    if path.starts_with(module_root) && !has_parent_component(path) {
        return Ok(());
    }
    Err(CliError::InvalidActionTarget {
        path: display_path(path),
        reason: "profile ledger path must stay under module root",
    })
}

fn ensure_existing_parent_components(path: &Path) -> Result<(), CliError> {
    let parent = path.parent().ok_or_else(|| CliError::InvalidActionTarget {
        path: display_path(path),
        reason: "profile ledger path has no parent",
    })?;
    let mut current = PathBuf::new();
    for component in parent.components() {
        current.push(component.as_os_str());
        if current.as_os_str().is_empty() {
            continue;
        }
        ensure_existing_directory_component(current.as_path())?;
    }
    Ok(())
}

fn ensure_existing_directory_component(path: &Path) -> Result<(), CliError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CliError::InvalidActionTarget {
            path: display_path(path),
            reason: "path component is a symlink",
        }),
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_metadata) => Err(CliError::InvalidActionTarget {
            path: display_path(path),
            reason: "path component is not a directory",
        }),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliError::Filesystem {
            path: display_path(path),
            source,
        }),
    }
}

fn ensure_profile_ledger_leaf_is_not_symlink(path: &Path) -> Result<(), CliError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CliError::InvalidActionTarget {
            path: display_path(path),
            reason: "profile ledger path is a symlink",
        }),
        Ok(_metadata) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliError::Filesystem {
            path: display_path(path),
            source,
        }),
    }
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}
