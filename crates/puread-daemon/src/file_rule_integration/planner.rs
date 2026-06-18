use std::path::{Path, PathBuf};

use puread_core::model::{ProfileKind, RiskLevel, RuleAction, RuleId};
use puread_core::path_expansion::{ExpandedPath, PathExpander};
use puread_rules::{RuleCategory, RuleDefinition};

use crate::DaemonError;

const MODULE_ANDROID_DIR: &str = "/data/adb/modules/puread";

/// dry-run 文件计划动作。
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DryRunFileAction {
    rule_id: String,
    category: RuleCategory,
    package: String,
    action: RuleAction,
    android_path: PathBuf,
    host_path: PathBuf,
}

impl DryRunFileAction {
    /// 返回规则 ID。
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }

    /// 返回规则类别。
    pub const fn category(&self) -> RuleCategory {
        self.category
    }

    /// 返回目标包名。
    pub fn package(&self) -> &str {
        &self.package
    }

    /// 返回计划动作。
    pub const fn action(&self) -> RuleAction {
        self.action
    }

    /// 返回 Android 逻辑路径。
    pub fn android_path(&self) -> &Path {
        &self.android_path
    }

    /// 返回宿主 dry-run 路径。
    pub fn host_path(&self) -> &Path {
        &self.host_path
    }
}

/// daemon 文件规则真实执行计划。
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApplyFileAction {
    rule_id: RuleId,
    category: RuleCategory,
    package: String,
    action: RuleAction,
    profile: ProfileKind,
    risk_level: RiskLevel,
    android_path: PathBuf,
    host_path: PathBuf,
}

impl ApplyFileAction {
    /// 返回规则 ID。
    pub const fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// 返回规则类别。
    pub const fn category(&self) -> RuleCategory {
        self.category
    }

    /// 返回目标包名。
    pub fn package(&self) -> &str {
        &self.package
    }

    /// 返回规则动作。
    pub const fn action(&self) -> RuleAction {
        self.action
    }

    /// 返回启用 profile。
    pub const fn profile(&self) -> ProfileKind {
        self.profile
    }

    /// 返回风险等级。
    pub const fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }

    /// 返回 Android 逻辑路径。
    pub fn android_path(&self) -> &Path {
        &self.android_path
    }

    /// 返回宿主执行路径。
    pub fn host_path(&self) -> &Path {
        &self.host_path
    }
}

#[derive(Debug, Clone)]
pub(super) struct FileRulePlanner {
    expander: PathExpander,
    rules: Vec<FileRule>,
}

impl FileRulePlanner {
    pub(super) fn new(android_root: PathBuf, rules: Vec<FileRule>) -> Result<Self, DaemonError> {
        let expander = PathExpander::new(android_root, MODULE_ANDROID_DIR).map_err(|source| {
            DaemonError::PathExpansion {
                rule_id: "path-expander-init".to_owned(),
                source,
            }
        })?;
        Ok(Self { expander, rules })
    }

    pub(super) fn dry_run_for_paths(
        &self,
        changed_paths: &[PathBuf],
    ) -> Result<Vec<DryRunFileAction>, DaemonError> {
        let mut actions = Vec::new();
        for rule in &self.rules {
            let expanded = self.expand_rule(rule)?;
            for path in expanded {
                if changed_paths
                    .iter()
                    .any(|changed| changed == path.host_path())
                {
                    push_unique_action(rule.plan_action(&path), &mut actions);
                }
            }
        }
        actions.sort_by(|left, right| {
            left.android_path
                .cmp(&right.android_path)
                .then_with(|| left.rule_id.cmp(&right.rule_id))
        });
        Ok(actions)
    }

    pub(super) fn apply_for_paths(
        &self,
        changed_paths: &[PathBuf],
    ) -> Result<Vec<ApplyFileAction>, DaemonError> {
        let mut actions = Vec::new();
        for rule in &self.rules {
            let expanded = self.expand_rule(rule)?;
            for path in expanded {
                if changed_paths
                    .iter()
                    .any(|changed| changed == path.host_path())
                {
                    push_unique_apply_action(rule.apply_action(&path), &mut actions);
                }
            }
        }
        actions.sort_by(|left, right| {
            left.android_path
                .cmp(&right.android_path)
                .then_with(|| left.rule_id.as_str().cmp(right.rule_id.as_str()))
        });
        Ok(actions)
    }

    pub(super) fn watch_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        for rule in &self.rules {
            if let Ok(paths) = self
                .expander
                .expand_template(rule.watch_template.as_str(), rule.package.as_str())
            {
                for path in paths {
                    push_unique_path(path.host_path().to_path_buf(), &mut roots);
                }
            }
        }
        roots.sort();
        roots
    }

    pub(super) const fn rule_count(&self) -> usize {
        self.rules.len()
    }

    fn expand_rule(&self, rule: &FileRule) -> Result<Vec<ExpandedPath>, DaemonError> {
        self.expander
            .expand_template(rule.template.as_str(), rule.package.as_str())
            .map_err(|source| DaemonError::PathExpansion {
                rule_id: rule.rule_id.as_str().to_owned(),
                source,
            })
    }
}

#[derive(Debug, Clone)]
pub(super) struct FileRule {
    rule_id: RuleId,
    category: RuleCategory,
    package: String,
    action: RuleAction,
    profile: ProfileKind,
    risk_level: RiskLevel,
    template: String,
    watch_template: String,
}

impl FileRule {
    pub(super) fn from_definition(rule: &RuleDefinition, template: &str) -> Option<Self> {
        if !rule.default_enabled() || !is_high_frequency_file_category(rule.category()) {
            return None;
        }
        Some(Self {
            rule_id: rule.id().clone(),
            category: rule.category(),
            package: rule.package().as_str().to_owned(),
            action: rule.action(),
            profile: rule.profile(),
            risk_level: rule.risk_level(),
            template: template.to_owned(),
            watch_template: watch_template(template),
        })
    }

    fn plan_action(&self, path: &ExpandedPath) -> DryRunFileAction {
        DryRunFileAction {
            rule_id: self.rule_id.as_str().to_owned(),
            category: self.category,
            package: self.package.clone(),
            action: self.action,
            android_path: path.android_path().to_path_buf(),
            host_path: path.host_path().to_path_buf(),
        }
    }

    fn apply_action(&self, path: &ExpandedPath) -> ApplyFileAction {
        ApplyFileAction {
            rule_id: self.rule_id.clone(),
            category: self.category,
            package: self.package.clone(),
            action: self.action,
            profile: self.profile,
            risk_level: self.risk_level,
            android_path: path.android_path().to_path_buf(),
            host_path: path.host_path().to_path_buf(),
        }
    }
}

const fn is_high_frequency_file_category(category: RuleCategory) -> bool {
    matches!(category, RuleCategory::FilePath | RuleCategory::SdkCache)
}

fn watch_template(template: &str) -> String {
    let Some((parent, _leaf)) = template.rsplit_once('/') else {
        return template.to_owned();
    };
    if parent.is_empty() {
        "/".to_owned()
    } else {
        parent.to_owned()
    }
}

fn push_unique_path(path: PathBuf, paths: &mut Vec<PathBuf>) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

fn push_unique_action(action: DryRunFileAction, actions: &mut Vec<DryRunFileAction>) {
    if !actions.contains(&action) {
        actions.push(action);
    }
}

fn push_unique_apply_action(action: ApplyFileAction, actions: &mut Vec<ApplyFileAction>) {
    if !actions.contains(&action) {
        actions.push(action);
    }
}
