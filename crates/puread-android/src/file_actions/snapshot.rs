use std::fs::{self, File, Metadata};
use std::io;

use puread_core::restore_ledger::OriginalFileType;

use crate::file_actions::error::FileActionError;
use crate::file_actions::target::FileActionTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TargetSnapshot {
    pub(super) original_type: OriginalFileType,
    pub(super) mode: u32,
    pub(super) uid: u32,
    pub(super) gid: u32,
    pub(super) nlink: u64,
    pub(super) selinux_context: Option<String>,
    pub(super) identity: TargetIdentity,
}

impl TargetSnapshot {
    pub(super) fn collect(target: &FileActionTarget) -> Result<Self, FileActionError> {
        match fs::symlink_metadata(target.host_path()) {
            Ok(metadata) => metadata_from_target(target, &metadata),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(Self::missing()),
            Err(source) => Err(FileActionError::io(target.host_path(), source)),
        }
    }

    pub(in crate::file_actions) fn collect_host_path(
        path: &std::path::Path,
    ) -> Result<Self, FileActionError> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err(
                FileActionError::rejected_target(path, "symlink target rejected"),
            ),
            Ok(metadata) => Ok(from_metadata(&metadata)),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(Self::missing()),
            Err(source) => Err(FileActionError::io(path, source)),
        }
    }

    pub(super) fn identity_changed_from_metadata(&self, metadata: &Metadata) -> bool {
        identity_changed(self.identity, metadata_identity(metadata))
    }

    pub(in crate::file_actions) fn collect_open_file(
        path: &std::path::Path,
        file: &File,
    ) -> Result<Self, FileActionError> {
        file.metadata()
            .map(|metadata| from_metadata(&metadata))
            .map_err(|source| FileActionError::io(path, source))
    }

    const fn missing() -> Self {
        Self {
            original_type: OriginalFileType::Missing,
            mode: 0,
            uid: 0,
            gid: 0,
            nlink: 0,
            selinux_context: None,
            identity: missing_identity(),
        }
    }
}

fn from_metadata(metadata: &Metadata) -> TargetSnapshot {
    TargetSnapshot {
        original_type: original_file_type(metadata),
        mode: metadata_mode(metadata),
        uid: metadata_uid(metadata),
        gid: metadata_gid(metadata),
        nlink: metadata_nlink(metadata),
        selinux_context: None,
        identity: metadata_identity(metadata),
    }
}

fn metadata_from_target(
    target: &FileActionTarget,
    metadata: &Metadata,
) -> Result<TargetSnapshot, FileActionError> {
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Err(FileActionError::rejected_target(
            target.android_path(),
            "symlink targets are rejected",
        ));
    }
    Ok(from_metadata(metadata))
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
