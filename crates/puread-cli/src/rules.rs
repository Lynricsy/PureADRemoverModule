use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use puread_rules::{RuleCategory, RuleDefinition, RuleDocument, RuleTarget, parse_rules_toml};
use serde::Serialize;

use crate::cli::{RulesCommand, RulesListKind, RulesSubcommand};
use crate::error::CliError;
use crate::json::{SCHEMA_VERSION, display_path, write_json};

#[derive(Debug, Serialize)]
struct RulesValidateDocument {
    schema_version: u8,
    command: &'static str,
    valid: bool,
    error_count: usize,
    rule_file_count: usize,
    rule_count: usize,
}

#[derive(Debug, Serialize)]
struct RulesListDocument {
    schema_version: u8,
    command: &'static str,
    kind: &'static str,
    rule_file_count: usize,
    rule_count: usize,
    rules: Vec<RuleListItem>,
}

#[derive(Debug, Serialize)]
struct RuleListItem {
    id: String,
    category: String,
    package: String,
    action: String,
    target: RuleListTarget,
    risk_level: String,
    default_enabled: bool,
    profile: String,
    observed_behavior: String,
    rollback_strategy: String,
    introduced_by: String,
    reviewed_at: String,
    source: String,
    source_file: Option<String>,
    zip_entry: Option<String>,
    source_line_or_pattern: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RuleListTarget {
    PathTemplate { target_template: String },
    Component { target_component: String },
    AppOp { appop: String, appop_mode: String },
    Unknown,
}

pub fn run_rules(command: RulesCommand) -> Result<(), CliError> {
    match command.command {
        RulesSubcommand::Validate(args) => run_validate(&args.paths),
        RulesSubcommand::List(args) => run_list(args.kind),
    }
}

fn run_validate(paths: &[PathBuf]) -> Result<(), CliError> {
    let rule_files = collect_rule_files_from_paths(paths)?;
    let documents = load_rule_documents(&rule_files)?;
    let document = RulesValidateDocument {
        schema_version: SCHEMA_VERSION,
        command: "rules_validate",
        valid: true,
        error_count: 0,
        rule_file_count: rule_files.len(),
        rule_count: count_rules(&documents),
    };
    write_json(&document)
}

fn run_list(kind: RulesListKind) -> Result<(), CliError> {
    let roots = default_list_roots(kind);
    let rule_files = collect_rule_files_from_paths(&roots)?;
    let documents = load_rule_documents(&rule_files)?;
    let rules = list_items(kind, &documents);
    let document = RulesListDocument {
        schema_version: SCHEMA_VERSION,
        command: "rules_list",
        kind: kind_name(kind),
        rule_file_count: rule_files.len(),
        rule_count: rules.len(),
        rules,
    };
    write_json(&document)
}

fn default_list_roots(kind: RulesListKind) -> Vec<PathBuf> {
    let workspace = workspace_root();
    match kind {
        RulesListKind::Files => vec![workspace.join("rules/common"), workspace.join("rules/apps")],
        RulesListKind::Sqlite => vec![workspace.join("rules/sqlite")],
    }
}

fn collect_rule_files_from_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    for path in paths {
        collect_rule_files_from_path(path, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_rule_files_from_path(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliError> {
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
        files.push(path.to_path_buf());
        return Ok(());
    }
    if metadata.is_dir() {
        return collect_rule_files_from_dir(path, files);
    }
    Err(CliError::RulesNotFileOrDirectory {
        path: display_path(path),
    })
}

fn collect_rule_files_from_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliError> {
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
            collect_rule_files_from_dir(&path, files)?;
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

fn count_rules(documents: &[RuleDocument]) -> usize {
    documents
        .iter()
        .map(|document| document.rules().len())
        .sum()
}

fn list_items(kind: RulesListKind, documents: &[RuleDocument]) -> Vec<RuleListItem> {
    let mut rules = documents
        .iter()
        .flat_map(RuleDocument::rules)
        .filter(|rule| matches_kind(kind, rule.category()))
        .map(rule_list_item)
        .collect::<Vec<_>>();
    rules.sort_by(|left, right| left.id.cmp(&right.id));
    rules
}

fn matches_kind(kind: RulesListKind, category: RuleCategory) -> bool {
    match kind {
        RulesListKind::Files => {
            matches!(category, RuleCategory::FilePath | RuleCategory::SdkCache)
        }
        RulesListKind::Sqlite => category == RuleCategory::Sqlite,
    }
}

fn rule_list_item(rule: &RuleDefinition) -> RuleListItem {
    let source = rule.source();
    RuleListItem {
        id: rule.id().as_str().to_owned(),
        category: rule.category().as_str().to_owned(),
        package: rule.package().as_str().to_owned(),
        action: rule.action().as_str().to_owned(),
        target: rule_target(rule.target()),
        risk_level: rule.risk_level().as_str().to_owned(),
        default_enabled: rule.default_enabled(),
        profile: rule.profile().as_str().to_owned(),
        observed_behavior: rule.observed_behavior().to_owned(),
        rollback_strategy: rule.rollback_strategy().as_str().to_owned(),
        introduced_by: rule.introduced_by().to_owned(),
        reviewed_at: rule.reviewed_at().to_owned(),
        source: source.source().to_owned(),
        source_file: source.source_file().map(ToOwned::to_owned),
        zip_entry: source.zip_entry().map(ToOwned::to_owned),
        source_line_or_pattern: source.line_or_pattern().to_owned(),
    }
}

fn rule_target(target: &RuleTarget) -> RuleListTarget {
    match target {
        RuleTarget::PathTemplate(path) => RuleListTarget::PathTemplate {
            target_template: path.as_str().to_owned(),
        },
        RuleTarget::Component(component) => RuleListTarget::Component {
            target_component: component.to_owned(),
        },
        RuleTarget::AppOp { op, mode } => RuleListTarget::AppOp {
            appop: op.to_owned(),
            appop_mode: mode.to_owned(),
        },
        _ => RuleListTarget::Unknown,
    }
}

const fn kind_name(kind: RulesListKind) -> &'static str {
    match kind {
        RulesListKind::Files => "files",
        RulesListKind::Sqlite => "sqlite",
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
