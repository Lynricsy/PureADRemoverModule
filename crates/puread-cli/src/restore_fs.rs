use std::fs;
use std::io::{Read as _, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Component, Path, PathBuf};

use crate::error::CliError;
use crate::json::display_path;

const O_NOFOLLOW: i32 = 0o400_000;

#[derive(Debug)]
pub struct RestoreRoot {
    path: PathBuf,
}

impl RestoreRoot {
    pub fn new(path: PathBuf) -> Result<Self, CliError> {
        guard_existing_directory_chain(path.as_path())?;
        Ok(Self { path })
    }

    pub fn map_android_path(&self, android_path: &str) -> PathBuf {
        self.path.join(android_path.trim_start_matches('/'))
    }

    pub fn guard_existing_path(&self, path: &Path) -> Result<(), CliError> {
        self.ensure_root_bound(path)?;
        guard_existing_parent_chain(path)?;
        reject_symlink(path)
    }

    pub fn ensure_parent(&self, path: &Path) -> Result<(), CliError> {
        self.ensure_root_bound(path)?;
        ensure_parent(path)
    }

    fn ensure_root_bound(&self, path: &Path) -> Result<(), CliError> {
        if path.starts_with(self.path.as_path()) {
            return Ok(());
        }
        out_of_root(path)
    }
}

#[derive(Debug)]
pub struct GuardedBackup {
    path: PathBuf,
}

impl GuardedBackup {
    pub fn new(backup_path: &str, backup_root: &Path) -> Result<Self, CliError> {
        let path = PathBuf::from(backup_path);
        if !path.starts_with(backup_root) || has_parent_component(path.as_path()) {
            return out_of_root(path.as_path());
        }
        guard_existing_directory_chain(backup_root)?;
        guard_existing_parent_chain(path.as_path())?;
        reject_symlink(path.as_path())?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

pub fn restore_content(
    path: &Path,
    backup: &GuardedBackup,
    root: &RestoreRoot,
) -> Result<(), CliError> {
    let metadata = symlink_metadata(backup.path())?;
    if metadata.is_dir() {
        return restore_directory(path, backup.path(), root);
    }
    if !metadata.is_file() {
        return Err(CliError::InvalidActionTarget {
            path: display_path(backup.path()),
            reason: "backup path is not a regular file",
        });
    }
    let content = read_no_follow(backup.path())?;
    write_file(path, &content, root)
}

pub fn restore_empty_file(path: &Path, root: &RestoreRoot) -> Result<(), CliError> {
    write_file(path, b"", root)
}

pub fn recreate_directory(path: &Path, root: &RestoreRoot) -> Result<(), CliError> {
    root.ensure_parent(path)?;
    ensure_directory_chain(path)
}

pub fn remove_placeholder(path: &Path, root: &RestoreRoot) -> Result<(), CliError> {
    root.guard_existing_path(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_metadata) => fs::remove_file(path),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(source),
    }
    .map_err(|source| CliError::Filesystem {
        path: display_path(path),
        source,
    })
}

pub fn set_mode(path: &Path, mode: u32, root: &RestoreRoot) -> Result<(), CliError> {
    root.guard_existing_path(path)?;
    let permissions = fs::Permissions::from_mode(mode);
    fs::set_permissions(path, permissions).map_err(|source| fs_error(path, source))
}

fn restore_directory(path: &Path, backup_path: &Path, root: &RestoreRoot) -> Result<(), CliError> {
    remove_placeholder(path, root)?;
    copy_dir(backup_path, path, root)
}

fn copy_dir(source: &Path, target: &Path, root: &RestoreRoot) -> Result<(), CliError> {
    root.ensure_parent(target)?;
    ensure_directory_chain(target)?;
    for entry_result in
        fs::read_dir(source).map_err(|source_error| fs_error(source, source_error))?
    {
        let entry = entry_result.map_err(|error| CliError::Filesystem {
            path: display_path(source),
            source: error,
        })?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = symlink_metadata(source_path.as_path())?;
        if metadata.is_dir() {
            copy_dir(source_path.as_path(), target_path.as_path(), root)?;
        } else if metadata.is_file() {
            write_file(
                target_path.as_path(),
                &read_no_follow(source_path.as_path())?,
                root,
            )?;
        }
    }
    Ok(())
}

fn write_file(path: &Path, content: &[u8], root: &RestoreRoot) -> Result<(), CliError> {
    root.ensure_parent(path)?;
    reject_symlink(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)
        .map_err(|source| fs_error(path, source))?;
    file.write_all(content)
        .map_err(|source| fs_error(path, source))
}

fn open_read_no_follow(path: &Path) -> Result<fs::File, CliError> {
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)
        .map_err(|source| fs_error(path, source))
}

fn read_no_follow(path: &Path) -> Result<Vec<u8>, CliError> {
    let mut file = open_read_no_follow(path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .map_err(|source| CliError::Filesystem {
            path: display_path(path),
            source,
        })?;
    Ok(content)
}

fn symlink_metadata(path: &Path) -> Result<fs::Metadata, CliError> {
    fs::symlink_metadata(path).map_err(|source| fs_error(path, source))
}

fn fs_error(path: &Path, source: std::io::Error) -> CliError {
    CliError::Filesystem {
        path: display_path(path),
        source,
    }
}

fn ensure_parent(path: &Path) -> Result<(), CliError> {
    let parent = path.parent().ok_or_else(|| CliError::InvalidActionTarget {
        path: display_path(path),
        reason: "target has no parent",
    })?;
    ensure_directory_chain(parent)
}

fn ensure_directory_chain(path: &Path) -> Result<(), CliError> {
    walk_directory_chain(path, true)
}

fn guard_existing_directory_chain(path: &Path) -> Result<(), CliError> {
    walk_directory_chain(path, false)
}

fn guard_existing_parent_chain(path: &Path) -> Result<(), CliError> {
    let parent = path.parent().ok_or_else(|| CliError::InvalidActionTarget {
        path: display_path(path),
        reason: "path has no parent",
    })?;
    guard_existing_directory_chain(parent)
}

fn walk_directory_chain(path: &Path, create_missing: bool) -> Result<(), CliError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::Prefix(_) => current.push(component.as_os_str()),
            Component::Normal(part) => {
                current.push(part);
                guard_directory_component(current.as_path(), create_missing)?;
            }
            Component::CurDir => {}
            Component::ParentDir => return out_of_root(path),
        }
    }
    Ok(())
}

fn guard_directory_component(path: &Path, create_missing: bool) -> Result<(), CliError> {
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
        Err(source) if source.kind() == std::io::ErrorKind::NotFound && create_missing => {
            fs::create_dir(path).map_err(|source| CliError::Filesystem {
                path: display_path(path),
                source,
            })
        }
        Err(source) => Err(CliError::Filesystem {
            path: display_path(path),
            source,
        }),
    }
}

fn reject_symlink(path: &Path) -> Result<(), CliError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CliError::InvalidActionTarget {
            path: display_path(path),
            reason: "path is a symlink",
        }),
        Ok(_metadata) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliError::Filesystem {
            path: display_path(path),
            source,
        }),
    }
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn out_of_root<T>(path: &Path) -> Result<T, CliError> {
    Err(CliError::RestorePathOutOfRoot {
        path: display_path(path),
    })
}
