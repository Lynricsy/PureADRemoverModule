use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use fs2::FileExt as _;

use crate::error::CliError;
use crate::json::display_path;

pub const LOCK_RELATIVE_PATH: &str = "run/puread.lock";
const O_NOFOLLOW: i32 = 0o400_000;

#[derive(Debug)]
pub struct GlobalLock {
    file: File,
}

impl GlobalLock {
    pub fn acquire(path: &Path) -> Result<Self, CliError> {
        ensure_safe_parent(path)?;
        ensure_lock_leaf_is_not_symlink(path)?;
        let file = open_lock_no_follow(path)?;
        file.try_lock_exclusive().map_err(|source| {
            if source.kind() == io::ErrorKind::WouldBlock {
                return CliError::LockAlreadyHeld {
                    path: display_path(path),
                };
            }
            CliError::Filesystem {
                path: display_path(path),
                source,
            }
        })?;
        Ok(Self { file })
    }
}

impl Drop for GlobalLock {
    fn drop(&mut self) {
        let _unlock_result = fs2::FileExt::unlock(&self.file);
    }
}

pub fn lock_path(module_root: &Path, override_path: Option<&Path>) -> Result<PathBuf, CliError> {
    let path =
        override_path.map_or_else(|| module_root.join(LOCK_RELATIVE_PATH), Path::to_path_buf);
    ensure_lock_inside_module(module_root, path.as_path())?;
    Ok(path)
}

pub fn lock_is_held(path: &Path) -> Result<bool, CliError> {
    let Some(file) = open_lock_file_if_exists(path)? else {
        return Ok(false);
    };
    match file.try_lock_exclusive() {
        Ok(()) => {
            fs2::FileExt::unlock(&file).map_err(|source| CliError::Filesystem {
                path: display_path(path),
                source,
            })?;
            Ok(false)
        }
        Err(source) if source.kind() == io::ErrorKind::WouldBlock => Ok(true),
        Err(source) => Err(CliError::Filesystem {
            path: display_path(path),
            source,
        }),
    }
}

fn open_lock_file_if_exists(path: &Path) -> Result<Option<File>, CliError> {
    ensure_lock_leaf_is_not_symlink(path)?;
    match open_existing_lock_no_follow(path) {
        Ok(file) => Ok(Some(file)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CliError::Filesystem {
            path: display_path(path),
            source,
        }),
    }
}

fn ensure_lock_inside_module(module_root: &Path, path: &Path) -> Result<(), CliError> {
    if path.starts_with(module_root) && !has_parent_component(path) {
        return Ok(());
    }
    Err(CliError::InvalidActionTarget {
        path: display_path(path),
        reason: "lock path must stay under module root",
    })
}

fn ensure_safe_parent(path: &Path) -> Result<(), CliError> {
    let parent = path.parent().ok_or_else(|| CliError::LockPathHasNoParent {
        path: display_path(path),
    })?;
    safe_create_dir_all(parent)
}

fn safe_create_dir_all(path: &Path) -> Result<(), CliError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if current.as_os_str().is_empty() {
            continue;
        }
        ensure_directory_component(current.as_path())?;
    }
    Ok(())
}

fn ensure_directory_component(path: &Path) -> Result<(), CliError> {
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
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
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

fn ensure_lock_leaf_is_not_symlink(path: &Path) -> Result<(), CliError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CliError::InvalidActionTarget {
            path: display_path(path),
            reason: "lock path is a symlink",
        }),
        Ok(_metadata) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliError::Filesystem {
            path: display_path(path),
            source,
        }),
    }
}

fn open_lock_no_follow(path: &Path) -> Result<File, CliError> {
    open_lock_options(true, path).map_err(|source| CliError::Filesystem {
        path: display_path(path),
        source,
    })
}

fn open_existing_lock_no_follow(path: &Path) -> io::Result<File> {
    open_lock_options(false, path)
}

fn open_lock_options(create: bool, path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(create)
        .truncate(false)
        .mode(0o600)
        .custom_flags(O_NOFOLLOW)
        .open(path)
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}
