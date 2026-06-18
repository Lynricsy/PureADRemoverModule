use std::fs::{self, File, Metadata};
use std::path::Path;

use puread_core::restore_ledger::OriginalFileType;

use crate::sqlite_actions::error::SqliteActionError;

/// `SQLite` 目标执行前元信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteTargetMetadata {
    /// 执行前文件类型。
    pub file_type: OriginalFileType,
    /// 执行前长度。
    pub len: Option<u64>,
    /// 执行前 mode。
    pub mode: u32,
    /// 执行前 uid。
    pub uid: u32,
    /// 执行前 gid。
    pub gid: u32,
    pub(super) nlink: u64,
    pub(super) identity: TargetIdentity,
}

impl SqliteTargetMetadata {
    pub(super) fn collect(path: &Path) -> Result<Self, SqliteActionError> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => Ok(from_metadata(&metadata)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                file_type: OriginalFileType::Missing,
                len: None,
                mode: 0,
                uid: 0,
                gid: 0,
                nlink: 0,
                identity: missing_identity(),
            }),
            Err(source) => Err(SqliteActionError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub(super) fn collect_open_file(path: &Path, file: &File) -> Result<Self, SqliteActionError> {
        file.metadata()
            .map(|metadata| from_metadata(&metadata))
            .map_err(|source| SqliteActionError::Io {
                path: path.to_path_buf(),
                source,
            })
    }
}

fn from_metadata(metadata: &Metadata) -> SqliteTargetMetadata {
    SqliteTargetMetadata {
        file_type: original_file_type(metadata),
        len: Some(metadata.len()),
        mode: metadata_mode(metadata),
        uid: metadata_uid(metadata),
        gid: metadata_gid(metadata),
        nlink: metadata_nlink(metadata),
        identity: metadata_identity(metadata),
    }
}

fn original_file_type(metadata: &Metadata) -> OriginalFileType {
    let file_type = metadata.file_type();
    if file_type.is_file() {
        OriginalFileType::File
    } else if file_type.is_dir() {
        OriginalFileType::Directory
    } else if file_type.is_symlink() {
        OriginalFileType::Symlink
    } else {
        OriginalFileType::Other
    }
}

#[cfg(unix)]
fn metadata_mode(metadata: &Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.mode()
}

#[cfg(not(unix))]
const fn metadata_mode(_metadata: &Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn metadata_uid(metadata: &Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.uid()
}

#[cfg(not(unix))]
const fn metadata_uid(_metadata: &Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn metadata_gid(metadata: &Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.gid()
}

#[cfg(unix)]
fn metadata_nlink(metadata: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink()
}

#[cfg(not(unix))]
const fn metadata_nlink(_metadata: &Metadata) -> u64 {
    1
}

#[cfg(not(unix))]
const fn metadata_gid(_metadata: &Metadata) -> u32 {
    0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
type TargetIdentity = FileIdentity;

#[cfg(not(unix))]
type TargetIdentity = ();

#[cfg(unix)]
fn metadata_identity(metadata: &Metadata) -> TargetIdentity {
    use std::os::unix::fs::MetadataExt;
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(not(unix))]
const fn metadata_identity(_metadata: &Metadata) -> TargetIdentity {}

#[cfg(unix)]
const fn missing_identity() -> TargetIdentity {
    FileIdentity {
        device: 0,
        inode: 0,
    }
}

#[cfg(not(unix))]
const fn missing_identity() -> TargetIdentity {}

#[cfg(unix)]
const fn has_identity(identity: TargetIdentity) -> bool {
    identity.device != 0 || identity.inode != 0
}

#[cfg(not(unix))]
const fn has_identity(_identity: TargetIdentity) -> bool {
    false
}

pub(super) fn identity_changed(original: TargetIdentity, current: TargetIdentity) -> bool {
    has_identity(original) && current != original
}
