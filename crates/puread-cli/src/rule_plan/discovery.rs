use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use puread_rules::{RuleDocument, parse_rules_toml};

use crate::error::CliError;
use crate::json::display_path;

const FILE_RULE_ALIAS: &str = "rules/files";

pub(super) fn resolve_rule_files(path: &Path) -> Result<Vec<PathBuf>, CliError> {
    if path == Path::new(FILE_RULE_ALIAS) {
        let workspace = workspace_root();
        let roots = [workspace.join("rules/common"), workspace.join("rules/apps")];
        return collect_rule_files_from_roots(&roots);
    }
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Err(CliError::MissingRules {
                path: display_path(path),
            });
        }
        Err(source) => {
            return Err(CliError::Filesystem {
                path: display_path(path),
                source,
            });
        }
    };
    if metadata.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if metadata.is_dir() {
        let roots = [path.to_path_buf()];
        return collect_rule_files_from_roots(&roots);
    }
    Err(CliError::RulesNotFileOrDirectory {
        path: display_path(path),
    })
}

fn collect_rule_files_from_roots(roots: &[PathBuf]) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    for root in roots {
        collect_rule_files(root, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_rule_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliError> {
    let mut entries = Vec::new();
    for entry_result in fs::read_dir(dir).map_err(|source| CliError::Filesystem {
        path: display_path(dir),
        source,
    })? {
        let entry = entry_result.map_err(|source| CliError::Filesystem {
            path: display_path(dir),
            source,
        })?;
        entries.push(entry.path());
    }
    entries.sort();
    for path in entries {
        let metadata = fs::metadata(&path).map_err(|source| CliError::Filesystem {
            path: display_path(&path),
            source,
        })?;
        if metadata.is_dir() {
            collect_rule_files(&path, files)?;
        } else if metadata.is_file() && path.extension() == Some(OsStr::new("toml")) {
            files.push(path);
        }
    }
    Ok(())
}

pub(super) fn load_rule_documents(rule_files: &[PathBuf]) -> Result<Vec<RuleDocument>, CliError> {
    rule_files
        .iter()
        .map(|path| {
            let content = fs::read_to_string(path).map_err(|source| CliError::RuleRead {
                path: display_path(path),
                source,
            })?;
            parse_rules_toml(&content).map_err(|source| CliError::RuleParse {
                path: display_path(path),
                source,
            })
        })
        .collect()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
