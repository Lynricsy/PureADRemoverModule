#[path = "target_host.rs"]
mod host;
#[path = "target_mapping.rs"]
mod mapping;
#[path = "target_path_kind.rs"]
mod path_kind;

use std::path::{Path, PathBuf};

use crate::sqlite_actions::error::SqliteActionError;

pub(super) use host::validate_host_target_path;

/// `SQLite` 动作目标路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteActionTarget {
    path: PathBuf,
    filesystem_root: PathBuf,
}

impl SqliteActionTarget {
    /// 通过 Android 逻辑路径和受控宿主路径构造目标。
    pub fn from_android_path(
        android_path: impl AsRef<Path>,
        host_path: impl AsRef<Path>,
        filesystem_root: impl AsRef<Path>,
    ) -> Result<Self, SqliteActionError> {
        let android_path = android_path.as_ref();
        let host_path = host_path.as_ref();
        let filesystem_root = filesystem_root.as_ref();
        path_kind::validate_android_database_path(android_path)?;
        mapping::validate_host_database_mapping(android_path, host_path, filesystem_root)?;
        Ok(Self {
            path: host_path.to_path_buf(),
            filesystem_root: filesystem_root.to_path_buf(),
        })
    }

    /// 返回目标路径。
    #[must_use]
    pub const fn path(&self) -> &PathBuf {
        &self.path
    }

    /// 返回受控宿主测试根。
    #[must_use]
    pub const fn filesystem_root(&self) -> &PathBuf {
        &self.filesystem_root
    }
}
