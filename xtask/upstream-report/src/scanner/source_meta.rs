use std::path::Path;
use std::process::Command;

use sha2::{Digest as _, Sha256};

use crate::error::{ReportError, command_failed, display_path, io_at};

pub(super) fn source_id(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| display_path(path), ToOwned::to_owned)
}

pub(super) fn sha256_file(path: &Path) -> Result<String, ReportError> {
    let bytes = std::fs::read(path).map_err(|source| io_at(path, source))?;
    Ok(sha256_bytes(&bytes))
}

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(super) fn git_value(dir: &Path, args: &[&str]) -> Result<String, ReportError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|source| io_at(dir, source))?;
    if !output.status.success() {
        return Ok("n/a".to_owned());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|source| ReportError::CommandUtf8 {
            path: display_path(dir),
            source,
        })
}

pub(super) fn run_required_command(
    program: &'static str,
    path: &Path,
    args: &[&str],
) -> Result<Vec<u8>, ReportError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|source| io_at(path, source))?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(command_failed(program, path, &output))
}
