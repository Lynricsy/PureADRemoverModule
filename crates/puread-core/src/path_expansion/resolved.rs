use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use super::PathExpansionError;
use super::validation::android_path_from_relative;

/// 已展开但尚未执行任何修改的路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedPath {
    android_path: PathBuf,
    host_path: PathBuf,
}

impl ExpandedPath {
    /// Android 设备上的逻辑目标路径。
    #[must_use]
    pub fn android_path(&self) -> &Path {
        &self.android_path
    }

    /// 测试或宿主 dry-run 环境中的真实映射路径。
    #[must_use]
    pub fn host_path(&self) -> &Path {
        &self.host_path
    }
}

#[derive(Debug, Clone)]
pub(super) struct PathResolver {
    filesystem_root: PathBuf,
}

impl PathResolver {
    pub(super) const fn new(filesystem_root: PathBuf) -> Self {
        Self { filesystem_root }
    }

    pub(super) fn resolve_existing(
        &self,
        android_path: &Path,
    ) -> Result<Option<ExpandedPath>, PathExpansionError> {
        let host_path = self.host_from_android(android_path);
        if !host_path.exists() {
            return Ok(None);
        }
        self.reject_symlink_escape(android_path, &host_path)?;
        Ok(Some(ExpandedPath {
            android_path: android_path.to_path_buf(),
            host_path,
        }))
    }

    pub(super) fn read_dir_or_empty(path: &Path) -> Result<Vec<fs::DirEntry>, PathExpansionError> {
        match fs::read_dir(path) {
            Ok(entries) => entries
                .map(|entry| {
                    entry.map_err(|source| PathExpansionError::Io {
                        path: path.to_path_buf(),
                        source,
                    })
                })
                .collect(),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(source) => Err(PathExpansionError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub(super) fn host_from_android(&self, android_path: &Path) -> PathBuf {
        let mut host_path = self.filesystem_root.clone();
        for component in android_path.components() {
            if let Component::Normal(segment) = component {
                host_path.push(segment);
            }
        }
        host_path
    }

    pub(super) fn collect_name_match(
        &self,
        entry: &fs::DirEntry,
        name: &str,
        matches: &mut Vec<ExpandedPath>,
    ) -> Result<(), PathExpansionError> {
        if entry.file_name() != OsStr::new(name) {
            return Ok(());
        }
        let android_path = self.android_from_host(&entry.path())?;
        if let Some(path) = self.resolve_existing(&android_path)? {
            matches.push(path);
        }
        Ok(())
    }

    fn reject_symlink_escape(
        &self,
        android_path: &Path,
        host_path: &Path,
    ) -> Result<(), PathExpansionError> {
        let canonical_root = Self::canonicalize(&self.filesystem_root)?;
        let canonical_host = Self::canonicalize(host_path)?;
        if canonical_host.starts_with(&canonical_root) {
            return Ok(());
        }
        Err(PathExpansionError::SymlinkEscape {
            android_path: android_path.to_path_buf(),
            host_path: host_path.to_path_buf(),
            root: canonical_root,
        })
    }

    fn canonicalize(path: &Path) -> Result<PathBuf, PathExpansionError> {
        fs::canonicalize(path).map_err(|source| PathExpansionError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    fn android_from_host(&self, host_path: &Path) -> Result<PathBuf, PathExpansionError> {
        let relative = host_path.strip_prefix(&self.filesystem_root).map_err(|_| {
            PathExpansionError::SymlinkEscape {
                android_path: PathBuf::from("/"),
                host_path: host_path.to_path_buf(),
                root: self.filesystem_root.clone(),
            }
        })?;
        Ok(android_path_from_relative(relative))
    }
}
