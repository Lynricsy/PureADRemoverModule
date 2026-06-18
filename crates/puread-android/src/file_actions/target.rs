use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::file_actions::error::FileActionError;

/// 已校验的文件动作目标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileActionTarget {
    android_path: PathBuf,
    host_path: PathBuf,
    filesystem_root: PathBuf,
}

impl FileActionTarget {
    /// 创建 Android 逻辑路径到宿主测试路径的安全映射。
    pub fn new(
        android_path: impl AsRef<Path>,
        host_path: impl AsRef<Path>,
        filesystem_root: impl AsRef<Path>,
    ) -> Result<Self, FileActionError> {
        let android_path = android_path.as_ref().to_path_buf();
        let host_path = host_path.as_ref().to_path_buf();
        let filesystem_root = filesystem_root.as_ref().to_path_buf();
        validate_android_path(&android_path)?;
        validate_host_mapping(&android_path, &host_path, &filesystem_root)?;
        Ok(Self {
            android_path,
            host_path,
            filesystem_root,
        })
    }

    /// 返回 Android 设备上的逻辑路径。
    #[must_use]
    pub fn android_path(&self) -> &Path {
        self.android_path.as_path()
    }

    /// 返回宿主临时根中的真实路径。
    #[must_use]
    pub fn host_path(&self) -> &Path {
        self.host_path.as_path()
    }

    /// 返回宿主临时根。
    #[must_use]
    pub fn filesystem_root(&self) -> &Path {
        self.filesystem_root.as_path()
    }
}

fn validate_android_path(path: &Path) -> Result<(), FileActionError> {
    if !path.is_absolute() {
        return Err(FileActionError::rejected_target(
            path,
            "android path must be absolute",
        ));
    }
    if has_bad_component(path) {
        return Err(FileActionError::rejected_target(
            path,
            "android path must not traverse",
        ));
    }
    if is_protected_root(path) || is_protected_subtree(path) {
        return Err(FileActionError::rejected_target(
            path,
            "protected root rejected",
        ));
    }
    Ok(())
}

fn validate_host_mapping(
    android_path: &Path,
    host_path: &Path,
    filesystem_root: &Path,
) -> Result<(), FileActionError> {
    if !host_path.is_absolute() || !filesystem_root.is_absolute() {
        return Err(FileActionError::rejected_target(
            android_path,
            "host paths must be absolute",
        ));
    }
    if has_bad_component(host_path) || has_bad_component(filesystem_root) {
        return Err(FileActionError::rejected_target(
            android_path,
            "host path must not traverse",
        ));
    }
    if !host_path.starts_with(filesystem_root) {
        return Err(FileActionError::rejected_target(
            android_path,
            "host path escapes filesystem root",
        ));
    }
    reject_symlink_escape(android_path, host_path, filesystem_root)
}

fn reject_symlink_escape(
    android_path: &Path,
    host_path: &Path,
    filesystem_root: &Path,
) -> Result<(), FileActionError> {
    if fs::symlink_metadata(host_path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(FileActionError::rejected_target(
            android_path,
            "symlink targets are rejected",
        ));
    }
    let existing = existing_path_for_canonical_check(host_path, filesystem_root);
    let root = canonicalize(filesystem_root)?;
    let resolved = canonicalize(existing)?;
    if resolved.starts_with(root) {
        return Ok(());
    }
    Err(FileActionError::rejected_target(
        android_path,
        "canonical host path escapes filesystem root",
    ))
}

fn canonicalize(path: &Path) -> Result<PathBuf, FileActionError> {
    fs::canonicalize(path).map_err(|source| FileActionError::io(path, source))
}

fn has_bad_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::CurDir | Component::Prefix(_)
        )
    })
}

fn is_protected_root(path: &Path) -> bool {
    ["/", "/data", "/sdcard", "/storage", "/system", "/vendor"]
        .iter()
        .any(|protected| path == Path::new(protected))
}

fn is_protected_subtree(path: &Path) -> bool {
    ["/data/adb", "/data/local/tmp"]
        .iter()
        .any(|protected| path == Path::new(protected) || path.starts_with(protected))
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
