use std::io;
use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("use --report-only for local upstream audits")]
    ReportOnlyRequired,
    #[error("input path not found: {path}")]
    InputMissing { path: String },
    #[error("input path must be a directory or zip file: {path}")]
    UnsupportedInput { path: String },
    #[error("zip input is not a file: {path}")]
    ZipNotFile { path: String },
    #[error("filesystem error at {path}: {source}")]
    Filesystem { path: String, source: io::Error },
    #[error("command `{program}` failed for {path}: {stderr}")]
    CommandFailed {
        program: &'static str,
        path: String,
        stderr: String,
    },
    #[error("failed to decode command output for {path}: {source}")]
    CommandUtf8 { path: String, source: FromUtf8Error },
    #[error("failed to serialize manifest JSON: {source}")]
    Json { source: serde_json::Error },
    #[error("failed to format timestamp: {source}")]
    TimeFormat { source: time::error::Format },
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub fn display_path(path: &std::path::Path) -> String {
    path.display().to_string()
}

pub fn io_at(path: &std::path::Path, source: io::Error) -> ReportError {
    ReportError::Filesystem {
        path: display_path(path),
        source,
    }
}

pub fn command_failed(
    program: &'static str,
    path: &std::path::Path,
    output: &std::process::Output,
) -> ReportError {
    ReportError::CommandFailed {
        program,
        path: display_path(path),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    }
}

pub fn input_missing(path: &std::path::Path) -> ReportError {
    ReportError::InputMissing {
        path: display_path(path),
    }
}

pub fn unsupported_input(path: &std::path::Path) -> ReportError {
    ReportError::UnsupportedInput {
        path: display_path(path),
    }
}

pub fn zip_not_file(path: &std::path::Path) -> ReportError {
    ReportError::ZipNotFile {
        path: display_path(path),
    }
}
