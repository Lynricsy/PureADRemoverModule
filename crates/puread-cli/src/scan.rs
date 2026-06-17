use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use puread_core::path_expansion::{ExpandedPath, PathExpander};
use puread_rules::{RuleDefinition, RuleDocument, RuleTarget, parse_rules_toml};
use serde::Serialize;

use crate::cli::ScanArgs;
use crate::error::CliError;
use crate::json::{SCHEMA_VERSION, display_path, write_json};

const FILE_RULE_ALIAS: &str = "rules/files";
const MODULE_ANDROID_DIR: &str = "/data/adb/modules/puread";

#[derive(Debug, Serialize)]
struct ScanDryRunDocument {
    schema_version: u8,
    mode: &'static str,
    dry_run: bool,
    root_path: String,
    rule_file_count: usize,
    action_count: usize,
    will_mutate: bool,
    actions: Vec<ScanPlannedAction>,
}

#[derive(Debug, Serialize)]
struct ScanPlannedAction {
    rule_id: String,
    category: String,
    package: String,
    action: String,
    profile: String,
    risk_level: String,
    default_enabled: bool,
    android_path: String,
    host_path: String,
    source: String,
    source_file: Option<String>,
    zip_entry: Option<String>,
}

pub fn run_scan(args: &ScanArgs) -> Result<(), CliError> {
    if !args.dry_run {
        return Err(CliError::RealScanUnsupported);
    }
    ensure_root_dir(args.root.as_path())?;
    let rule_files = resolve_rule_files(args.rules.as_path())?;
    let documents = load_rule_documents(&rule_files)?;
    let actions = plan_scan(args.root.as_path(), &documents)?;
    let document = ScanDryRunDocument {
        schema_version: SCHEMA_VERSION,
        mode: "dry_run",
        dry_run: true,
        root_path: display_path(args.root.as_path()),
        rule_file_count: rule_files.len(),
        action_count: actions.len(),
        will_mutate: false,
        actions,
    };
    write_json(&document)
}

fn ensure_root_dir(path: &Path) -> Result<(), CliError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Err(CliError::MissingRoot {
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
    if metadata.is_dir() {
        return Ok(());
    }
    Err(CliError::RootNotDirectory {
        path: display_path(path),
    })
}

fn resolve_rule_files(path: &Path) -> Result<Vec<PathBuf>, CliError> {
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

fn load_rule_documents(rule_files: &[PathBuf]) -> Result<Vec<RuleDocument>, CliError> {
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

fn plan_scan(root: &Path, documents: &[RuleDocument]) -> Result<Vec<ScanPlannedAction>, CliError> {
    let expander = PathExpander::new(root.to_path_buf(), MODULE_ANDROID_DIR).map_err(|source| {
        CliError::PathExpansion {
            rule_id: "path-expander-init".to_owned(),
            source,
        }
    })?;
    let mut actions = Vec::new();
    for rule in documents.iter().flat_map(RuleDocument::rules) {
        append_rule_actions(rule, &expander, &mut actions)?;
    }
    actions.sort_by(|left, right| {
        left.android_path
            .cmp(&right.android_path)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    Ok(actions)
}

fn append_rule_actions(
    rule: &RuleDefinition,
    expander: &PathExpander,
    actions: &mut Vec<ScanPlannedAction>,
) -> Result<(), CliError> {
    if !rule.default_enabled() {
        return Ok(());
    }
    let RuleTarget::PathTemplate(template) = rule.target() else {
        return Ok(());
    };
    let paths = expander
        .expand_template(template.as_str(), rule.package().as_str())
        .map_err(|source| CliError::PathExpansion {
            rule_id: rule.id().as_str().to_owned(),
            source,
        })?;
    actions.extend(paths.iter().map(|path| planned_action(rule, path)));
    Ok(())
}

fn planned_action(rule: &RuleDefinition, path: &ExpandedPath) -> ScanPlannedAction {
    let source = rule.source();
    ScanPlannedAction {
        rule_id: rule.id().as_str().to_owned(),
        category: rule.category().as_str().to_owned(),
        package: rule.package().as_str().to_owned(),
        action: rule.action().as_str().to_owned(),
        profile: rule.profile().as_str().to_owned(),
        risk_level: rule.risk_level().as_str().to_owned(),
        default_enabled: rule.default_enabled(),
        android_path: display_path(path.android_path()),
        host_path: display_path(path.host_path()),
        source: source.source().to_owned(),
        source_file: source.source_file().map(ToOwned::to_owned),
        zip_entry: source.zip_entry().map(ToOwned::to_owned),
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
