use std::fmt::Write as _;
use std::fs::{self, Metadata};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::profiles::ProfileError;
use crate::profiles::xml_bool::{read_bool_value, rewrite_bool_value};
use crate::profiles::xml_hooks;
use crate::secure_fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct XmlBoolMutation {
    pub original_value: bool,
    pub original_sha256: String,
    pub backup_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct XmlBoolPlan {
    pub original_value: bool,
    pub(super) original_sha256: String,
    pub(super) backup_path: PathBuf,
    identity: FileIdentity,
    original: Vec<u8>,
    updated: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    len: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
}

pub(super) fn plan_bool(
    path: &Path,
    key: &str,
    value: bool,
    backup_dir: &Path,
    rule_id: &str,
) -> Result<XmlBoolPlan, ProfileError> {
    validate_backup_rule_id(rule_id)?;
    let mut source = secure_fs::open_read_no_follow(path)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    let mut original = Vec::new();
    source
        .read_to_end(&mut original)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    let metadata = source
        .metadata()
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    let original_sha256 = sha256_hex(&original);
    let original_value = read_bool_value(&original, key, path)?;
    let backup_path = backup_path_for(backup_dir, rule_id);
    let updated = rewrite_bool_value(&original, key, value, path)?;
    Ok(XmlBoolPlan {
        original_value,
        original_sha256,
        backup_path,
        identity: FileIdentity::from_metadata(&metadata),
        original,
        updated,
    })
}

pub(super) fn commit_bool(
    path: &Path,
    backup_dir: &Path,
    plan: &XmlBoolPlan,
) -> Result<XmlBoolMutation, ProfileError> {
    secure_fs::create_dir_all_no_symlink(backup_dir)
        .map_err(|source| ProfileError::io(backup_dir.to_path_buf(), source))?;
    xml_hooks::before_commit_open(path)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    let mut target = open_matching_target(path, plan)?;
    let mut backup = secure_fs::create_new_no_follow(&plan.backup_path)
        .map_err(|source| ProfileError::io(plan.backup_path.clone(), source))?;
    backup
        .write_all(&plan.original)
        .map_err(|source| ProfileError::io(plan.backup_path.clone(), source))?;
    target
        .set_len(0)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    target
        .seek(SeekFrom::Start(0))
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    target
        .write_all(&plan.updated)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    Ok(XmlBoolMutation {
        original_value: plan.original_value,
        original_sha256: plan.original_sha256.clone(),
        backup_path: plan.backup_path.clone(),
    })
}

pub(super) fn preflight_bool_commit(
    path: &Path,
    backup_dir: &Path,
    plan: &XmlBoolPlan,
) -> Result<(), ProfileError> {
    let _target = open_matching_target(path, plan)?;
    ensure_backup_ready(backup_dir, &plan.backup_path)
}

fn open_matching_target(path: &Path, plan: &XmlBoolPlan) -> Result<std::fs::File, ProfileError> {
    let target = secure_fs::open_read_write_no_follow(path)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    let metadata = target
        .metadata()
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    if FileIdentity::from_metadata(&metadata) != plan.identity {
        return Err(ProfileError::io(
            path.to_path_buf(),
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "profile XML file changed between plan and commit",
            ),
        ));
    }
    Ok(target)
}

fn ensure_backup_ready(backup_dir: &Path, backup_path: &Path) -> Result<(), ProfileError> {
    secure_fs::ensure_parent_no_symlink(backup_dir)
        .map_err(|source| ProfileError::io(backup_dir.to_path_buf(), source))?;
    match fs::symlink_metadata(backup_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(ProfileError::io(backup_dir.to_path_buf(), not_directory()))
        }
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_metadata) => Err(ProfileError::io(backup_dir.to_path_buf(), not_directory())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ProfileError::io(backup_dir.to_path_buf(), error)),
    }?;
    match fs::symlink_metadata(backup_path) {
        Ok(_metadata) => Err(ProfileError::io(
            backup_path.to_path_buf(),
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "backup path already exists",
            ),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ProfileError::io(backup_path.to_path_buf(), error)),
    }
}

fn not_directory() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "backup directory must be a real directory",
    )
}

pub(super) fn restore_from_backup(path: &Path, backup_path: &Path) -> Result<(), ProfileError> {
    let mut backup = secure_fs::open_read_no_follow(backup_path)
        .map_err(|source| ProfileError::io(backup_path.to_path_buf(), source))?;
    let mut content = Vec::new();
    backup
        .read_to_end(&mut content)
        .map_err(|source| ProfileError::io(backup_path.to_path_buf(), source))?;
    let mut target = secure_fs::open_read_write_no_follow(path)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    target
        .set_len(0)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    target
        .seek(SeekFrom::Start(0))
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))?;
    target
        .write_all(&content)
        .map_err(|source| ProfileError::io(path.to_path_buf(), source))
}

fn backup_path_for(backup_dir: &Path, rule_id: &str) -> PathBuf {
    backup_dir.join(format!("{rule_id}.xml.bak"))
}

fn validate_backup_rule_id(rule_id: &str) -> Result<(), ProfileError> {
    if rule_id.is_empty()
        || rule_id.len() > 96
        || !rule_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(ProfileError::invalid_rule(
            "rule_id",
            rule_id,
            "rule id must be a short ASCII token for backup filenames",
        ));
    }
    Ok(())
}

impl FileIdentity {
    fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            dev: metadata.dev(),
            #[cfg(unix)]
            ino: metadata.ino(),
        }
    }
}

fn sha256_hex(content: &[u8]) -> String {
    let digest = Sha256::digest(content);
    digest.iter().fold(String::new(), |mut output, byte| {
        let _ = write!(output, "{byte:02x}");
        output
    })
}

#[cfg(test)]
mod tests;
