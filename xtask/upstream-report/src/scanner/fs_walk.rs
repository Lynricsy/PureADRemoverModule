use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{ReportError, io_at};

pub(super) fn is_zip_path(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("zip"))
}

pub(super) fn sorted_children(dir: &Path) -> Result<Vec<PathBuf>, ReportError> {
    let mut children = Vec::new();
    for entry_result in fs::read_dir(dir).map_err(|source| io_at(dir, source))? {
        let entry = entry_result.map_err(|source| io_at(dir, source))?;
        children.push(entry.path());
    }
    children.sort();
    Ok(children)
}

pub(super) fn collect_files(dir: &Path) -> Result<Vec<PathBuf>, ReportError> {
    let mut files = Vec::new();
    collect_files_into(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files_into(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), ReportError> {
    for path in sorted_children(dir)? {
        if is_git_internal_path(&path) {
            continue;
        }
        let metadata = fs::metadata(&path).map_err(|source| io_at(&path, source))?;
        if metadata.is_dir() {
            collect_files_into(&path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

pub(super) fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_error| path.display().to_string(),
        |relative| relative.display().to_string(),
    )
}

fn is_git_internal_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == OsStr::new(".git"))
}
