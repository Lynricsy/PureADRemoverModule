use std::path::Path;

use crate::error::{ReportError, display_path};
use crate::scanner::source_meta::run_required_command;

pub(super) fn zip_entries(zip_path: &Path) -> Result<Vec<String>, ReportError> {
    let output = run_required_command("unzip", zip_path, &["-Z1", &display_path(zip_path)])?;
    let text = String::from_utf8(output).map_err(|source| ReportError::CommandUtf8 {
        path: display_path(zip_path),
        source,
    })?;
    Ok(text.lines().map(ToOwned::to_owned).collect())
}

pub(super) fn zip_entry_content(zip_path: &Path, entry: &str) -> Result<Vec<u8>, ReportError> {
    run_required_command("unzip", zip_path, &["-p", &display_path(zip_path), entry])
}
