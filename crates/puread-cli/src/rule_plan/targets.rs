use std::path::{Path, PathBuf};

use puread_core::path_expansion::{ExpandedPath, PathExpander};
use puread_rules::{RuleCategory, RuleDefinition, RuleDocument, RuleTarget};

use super::PlannedAction;
use crate::error::CliError;
use crate::json::display_path;

pub(super) fn plan_actions(
    root: &Path,
    documents: &[RuleDocument],
    module_android_dir: &Path,
    profile: Option<&str>,
) -> Result<Vec<PlannedAction>, CliError> {
    let expander = PathExpander::new(root.to_path_buf(), module_android_dir).map_err(|source| {
        CliError::PathExpansion {
            rule_id: "path-expander-init".to_owned(),
            source,
        }
    })?;
    let mut actions = Vec::new();
    for rule in documents.iter().flat_map(RuleDocument::rules) {
        append_rule_actions(root, rule, &expander, profile, &mut actions)?;
    }
    actions.sort_by(|left, right| {
        left.android_path
            .cmp(&right.android_path)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    Ok(actions)
}

fn append_rule_actions(
    root: &Path,
    rule: &RuleDefinition,
    expander: &PathExpander,
    profile: Option<&str>,
    actions: &mut Vec<PlannedAction>,
) -> Result<(), CliError> {
    if profile.is_some_and(|value| rule.profile().as_str() != value) {
        return Ok(());
    }
    if profile.is_none() && !rule.default_enabled() {
        return Ok(());
    }
    append_target_actions(root, rule, expander, actions)
}

fn append_target_actions(
    root: &Path,
    rule: &RuleDefinition,
    expander: &PathExpander,
    actions: &mut Vec<PlannedAction>,
) -> Result<(), CliError> {
    match rule.target() {
        RuleTarget::PathTemplate(template) => {
            append_path_template_actions(root, rule, expander, template.as_str(), actions)?;
        }
        RuleTarget::Component(component) => actions.push(planned_component_action(rule, component)),
        RuleTarget::AppOp { op, mode } => actions.push(planned_appop_action(rule, op, mode)),
        _ => {}
    }
    Ok(())
}

fn append_path_template_actions(
    root: &Path,
    rule: &RuleDefinition,
    expander: &PathExpander,
    template: &str,
    actions: &mut Vec<PlannedAction>,
) -> Result<(), CliError> {
    if rule.category() == RuleCategory::RomProfile {
        let path = ExpandedPlanPath::from_android(root, template);
        actions.push(planned_path_action(rule, &path));
        return Ok(());
    }
    let paths = expander
        .expand_template(template, rule.package().as_str())
        .map_err(|source| CliError::PathExpansion {
            rule_id: rule.id().as_str().to_owned(),
            source,
        })?;
    actions.extend(paths.iter().map(|path| planned_path_action(rule, path)));
    Ok(())
}

fn planned_path_action(rule: &RuleDefinition, path: &dyn PlanPath) -> PlannedAction {
    let source = rule.source();
    PlannedAction {
        rule_id: rule.id().as_str().to_owned(),
        category: rule.category().as_str().to_owned(),
        package: rule.package().as_str().to_owned(),
        action: rule.action().as_str().to_owned(),
        schedule: rule.schedule().map(ToOwned::to_owned),
        profile: rule.profile().as_str().to_owned(),
        risk_level: rule.risk_level().as_str().to_owned(),
        default_enabled: rule.default_enabled(),
        android_path: display_path(path.android_path()),
        host_path: display_path(path.host_path()),
        target_kind: target_kind(rule.category()).to_owned(),
        component: None,
        appop: None,
        appop_mode: None,
        source: source.source().to_owned(),
        source_file: source.source_file().map(ToOwned::to_owned),
        zip_entry: source.zip_entry().map(ToOwned::to_owned),
    }
}

trait PlanPath {
    fn android_path(&self) -> &Path;
    fn host_path(&self) -> &Path;
}

impl PlanPath for ExpandedPath {
    fn android_path(&self) -> &Path {
        self.android_path()
    }

    fn host_path(&self) -> &Path {
        self.host_path()
    }
}

struct ExpandedPlanPath {
    android_path: PathBuf,
    host_path: PathBuf,
}

impl ExpandedPlanPath {
    fn from_android(root: &Path, android_path: &str) -> Self {
        let android_path = PathBuf::from(android_path);
        let host_path = root.join(android_path.strip_prefix("/").unwrap_or(&android_path));
        Self {
            android_path,
            host_path,
        }
    }
}

impl PlanPath for ExpandedPlanPath {
    fn android_path(&self) -> &Path {
        &self.android_path
    }

    fn host_path(&self) -> &Path {
        &self.host_path
    }
}

fn planned_component_action(rule: &RuleDefinition, component: &str) -> PlannedAction {
    let source = rule.source();
    PlannedAction {
        rule_id: rule.id().as_str().to_owned(),
        category: rule.category().as_str().to_owned(),
        package: rule.package().as_str().to_owned(),
        action: rule.action().as_str().to_owned(),
        schedule: rule.schedule().map(ToOwned::to_owned),
        profile: rule.profile().as_str().to_owned(),
        risk_level: rule.risk_level().as_str().to_owned(),
        default_enabled: rule.default_enabled(),
        android_path: component.to_owned(),
        host_path: String::new(),
        target_kind: "component".to_owned(),
        component: Some(component.to_owned()),
        appop: None,
        appop_mode: None,
        source: source.source().to_owned(),
        source_file: source.source_file().map(ToOwned::to_owned),
        zip_entry: source.zip_entry().map(ToOwned::to_owned),
    }
}

fn planned_appop_action(rule: &RuleDefinition, op: &str, mode: &str) -> PlannedAction {
    let source = rule.source();
    PlannedAction {
        rule_id: rule.id().as_str().to_owned(),
        category: rule.category().as_str().to_owned(),
        package: rule.package().as_str().to_owned(),
        action: rule.action().as_str().to_owned(),
        schedule: rule.schedule().map(ToOwned::to_owned),
        profile: rule.profile().as_str().to_owned(),
        risk_level: rule.risk_level().as_str().to_owned(),
        default_enabled: rule.default_enabled(),
        android_path: format!("{}:{op}", rule.package().as_str()),
        host_path: String::new(),
        target_kind: "appop".to_owned(),
        component: None,
        appop: Some(op.to_owned()),
        appop_mode: Some(mode.to_owned()),
        source: source.source().to_owned(),
        source_file: source.source_file().map(ToOwned::to_owned),
        zip_entry: source.zip_entry().map(ToOwned::to_owned),
    }
}

const fn target_kind(category: RuleCategory) -> &'static str {
    match category {
        RuleCategory::RomProfile => "rom",
        RuleCategory::FilePath | RuleCategory::SdkCache | RuleCategory::Sqlite => "path",
        RuleCategory::Component => "component",
        RuleCategory::AppOps => "appop",
        _ => "unknown",
    }
}
