use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use puread_rules::{RuleCategory, RuleDefinition, RuleDocument, RuleTarget, parse_rules_toml};

use crate::DaemonError;
use crate::file_rule_integration::planner::FileRule;

pub(super) fn collect_file_rules(rule_roots: &[PathBuf]) -> Result<Vec<FileRule>, DaemonError> {
    let documents = load_documents(rule_roots)?;
    Ok(documents
        .iter()
        .flat_map(RuleDocument::rules)
        .filter_map(file_rule_from_definition)
        .collect())
}

pub(super) fn count_skipped_rules(rule_roots: &[PathBuf]) -> Result<usize, DaemonError> {
    let documents = load_documents(rule_roots)?;
    Ok(documents
        .iter()
        .flat_map(RuleDocument::rules)
        .filter(|rule| !is_high_frequency_file_category(rule.category()))
        .count())
}

fn load_documents(rule_roots: &[PathBuf]) -> Result<Vec<RuleDocument>, DaemonError> {
    let rule_files = collect_rule_files_from_roots(rule_roots)?;
    rule_files
        .iter()
        .map(|path| {
            let content = fs::read_to_string(path).map_err(|source| DaemonError::RuleRead {
                path: path.clone(),
                source,
            })?;
            parse_rules_toml(&content).map_err(|source| DaemonError::RuleParse {
                path: path.clone(),
                source,
            })
        })
        .collect()
}

fn collect_rule_files_from_roots(roots: &[PathBuf]) -> Result<Vec<PathBuf>, DaemonError> {
    let mut files = Vec::new();
    for root in roots {
        ensure_rule_dir(root)?;
        collect_rule_files(root, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_rule_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), DaemonError> {
    let mut entries = Vec::new();
    for entry_result in fs::read_dir(dir).map_err(|source| DaemonError::RuleReadDir {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry_result.map_err(|source| DaemonError::RuleReadDir {
            path: dir.to_path_buf(),
            source,
        })?;
        entries.push(entry.path());
    }
    entries.sort();
    for path in entries {
        let metadata = fs::metadata(&path).map_err(|source| DaemonError::RuleMetadata {
            path: path.clone(),
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

fn file_rule_from_definition(rule: &RuleDefinition) -> Option<FileRule> {
    let RuleTarget::PathTemplate(template) = rule.target() else {
        return None;
    };
    FileRule::from_definition(rule, template.as_str())
}

fn ensure_rule_dir(path: &Path) -> Result<(), DaemonError> {
    let metadata = fs::metadata(path).map_err(|source| DaemonError::RuleRootMissing {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.is_dir() {
        return Ok(());
    }
    Err(DaemonError::RuleRootNotDirectory {
        path: path.to_path_buf(),
    })
}

const fn is_high_frequency_file_category(category: RuleCategory) -> bool {
    matches!(category, RuleCategory::FilePath | RuleCategory::SdkCache)
}
