use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::CliError;
use crate::json::display_path;

mod discovery;
mod targets;

const DEFAULT_MODULE_ANDROID_DIR: &str = "/data/adb/modules/PureAD";

#[derive(Debug, Clone, Serialize)]
pub struct PlannedAction {
    pub rule_id: String,
    pub category: String,
    pub package: String,
    pub action: String,
    pub schedule: Option<String>,
    pub profile: String,
    pub risk_level: String,
    pub default_enabled: bool,
    pub android_path: String,
    pub host_path: String,
    pub target_kind: String,
    pub component: Option<String>,
    pub appop: Option<String>,
    pub appop_mode: Option<String>,
    pub source: String,
    pub source_file: Option<String>,
    pub zip_entry: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActionPlan {
    rule_file_count: usize,
    actions: Vec<PlannedAction>,
}

impl ActionPlan {
    pub fn new(
        root: &Path,
        rules_path: &Path,
        module_root: Option<&Path>,
        profile: Option<&str>,
    ) -> Result<Self, CliError> {
        let rule_files = discovery::resolve_rule_files(rules_path)?;
        let documents = discovery::load_rule_documents(&rule_files)?;
        let module_android_dir = module_android_dir(root, module_root);
        let actions =
            targets::plan_actions(root, &documents, module_android_dir.as_path(), profile)?;
        Ok(Self {
            rule_file_count: rule_files.len(),
            actions,
        })
    }

    pub const fn rule_file_count(&self) -> usize {
        self.rule_file_count
    }

    pub fn actions(&self) -> &[PlannedAction] {
        &self.actions
    }
}

fn module_android_dir(root: &Path, module_root: Option<&Path>) -> PathBuf {
    let Some(module_root) = module_root else {
        return PathBuf::from(DEFAULT_MODULE_ANDROID_DIR);
    };
    let relative = match module_root.strip_prefix(root) {
        Ok(relative) => relative,
        Err(_source) => {
            if module_root.is_absolute() {
                return module_root.to_path_buf();
            }
            return PathBuf::from(DEFAULT_MODULE_ANDROID_DIR);
        }
    };
    let mut android_path = PathBuf::from("/");
    android_path.push(relative);
    android_path
}

pub fn ensure_root_dir(path: &Path) -> Result<(), CliError> {
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
