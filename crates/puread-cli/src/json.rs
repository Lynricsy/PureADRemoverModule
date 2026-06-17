use std::io::{self, Write as _};
use std::path::Path;

use serde::Serialize;

use crate::error::CliError;

pub const SCHEMA_VERSION: u8 = 1;

pub fn write_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)
        .map_err(|source| CliError::JsonWrite { source })?;
    stdout
        .write_all(b"\n")
        .map_err(|source| CliError::OutputWrite { source })
}

pub fn display_path(path: &Path) -> String {
    path.display().to_string()
}
