use puread_core::model::{PackageName, RootPath};

use crate::RuleCategory;
use crate::error::RuleParseError;
use crate::raw::RawRule;
use crate::validation::require_text;

/// 规则目标。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleTarget {
    /// Android 路径模板。
    PathTemplate(RootPath),
    /// Android 组件名。
    Component(String),
    /// `AppOps` 操作和模式。
    AppOp {
        /// `AppOps` 操作名。
        op: String,
        /// `AppOps` 模式。
        mode: String,
    },
}

pub(crate) fn build_target(
    category: RuleCategory,
    package: &PackageName,
    raw: &RawRule,
) -> Result<RuleTarget, RuleParseError> {
    match category {
        RuleCategory::FilePath | RuleCategory::SdkCache | RuleCategory::Sqlite => {
            path_target(category, package, raw)
        }
        RuleCategory::RomProfile => rom_target(category, raw),
        RuleCategory::Component => component_target(category, package, raw),
        RuleCategory::AppOps => appop_target(category, raw),
    }
}

fn path_target(
    category: RuleCategory,
    package: &PackageName,
    raw: &RawRule,
) -> Result<RuleTarget, RuleParseError> {
    reject_non_path_fields(category, raw)?;
    let template = required_target_template(category, raw)?;
    let path = parse_package_scoped_path(category, package, template)?;
    Ok(RuleTarget::PathTemplate(path))
}

fn rom_target(category: RuleCategory, raw: &RawRule) -> Result<RuleTarget, RuleParseError> {
    reject_non_path_fields(category, raw)?;
    let template = required_target_template(category, raw)?;
    if !template.starts_with("/data/system/") && !template.starts_with("/data/misc/") {
        return invalid_target(
            category,
            "rom target must stay under /data/system or /data/misc",
        );
    }
    Ok(RuleTarget::PathTemplate(RootPath::parse(template)?))
}

fn component_target(
    category: RuleCategory,
    package: &PackageName,
    raw: &RawRule,
) -> Result<RuleTarget, RuleParseError> {
    if raw.target_template.is_some() || raw.appop.is_some() || raw.appop_mode.is_some() {
        return invalid_target(category, "component rule must only define target_component");
    }
    let Some(component) = raw.target_component.as_deref() else {
        return invalid_target(category, "component rule requires target_component");
    };
    validate_component(package, component)?;
    Ok(RuleTarget::Component(component.to_owned()))
}

fn appop_target(category: RuleCategory, raw: &RawRule) -> Result<RuleTarget, RuleParseError> {
    if raw.target_template.is_some() || raw.target_component.is_some() {
        return invalid_target(
            category,
            "appops rule must only define appop and appop_mode",
        );
    }
    let Some(op) = raw.appop.as_deref() else {
        return invalid_target(category, "appops rule requires appop");
    };
    let Some(mode) = raw.appop_mode.as_deref() else {
        return invalid_target(category, "appops rule requires appop_mode");
    };
    validate_appop(op)?;
    validate_appop_mode(mode)?;
    Ok(RuleTarget::AppOp {
        op: op.to_owned(),
        mode: mode.to_owned(),
    })
}

fn required_target_template(category: RuleCategory, raw: &RawRule) -> Result<&str, RuleParseError> {
    let Some(template) = raw.target_template.as_deref() else {
        return invalid_target(category, "path-like rule requires target_template");
    };
    require_text("target_template", template)?;
    Ok(template)
}

fn parse_package_scoped_path(
    category: RuleCategory,
    package: &PackageName,
    template: &str,
) -> Result<RootPath, RuleParseError> {
    let path = RootPath::parse(template)?;
    if is_package_scoped_template(template, package.as_str()) {
        return Ok(path);
    }
    invalid_target(
        category,
        "path target must stay under the target package scope",
    )
}

const fn reject_non_path_fields(
    category: RuleCategory,
    raw: &RawRule,
) -> Result<(), RuleParseError> {
    if raw.target_component.is_some() || raw.appop.is_some() || raw.appop_mode.is_some() {
        return invalid_target(
            category,
            "path-like rule must not define component or appops fields",
        );
    }
    Ok(())
}

fn validate_component(package: &PackageName, component: &str) -> Result<(), RuleParseError> {
    require_text("target_component", component)?;
    let Some((owner, name)) = component.split_once('/') else {
        return invalid_target(
            RuleCategory::Component,
            "component must include package/name",
        );
    };
    if owner != package.as_str() || name.trim().is_empty() || name.contains(char::is_whitespace) {
        return invalid_target(RuleCategory::Component, "component must belong to package");
    }
    Ok(())
}

fn validate_appop(op: &str) -> Result<(), RuleParseError> {
    require_text("appop", op)?;
    if op.chars().all(|ch| ch.is_ascii_uppercase() || ch == '_') {
        return Ok(());
    }
    invalid_target(RuleCategory::AppOps, "appop must be an uppercase token")
}

fn validate_appop_mode(mode: &str) -> Result<(), RuleParseError> {
    require_text("appop_mode", mode)?;
    match mode {
        "allow" | "ignore" | "deny" | "default" => Ok(()),
        _ => invalid_target(RuleCategory::AppOps, "unsupported appop_mode"),
    }
}

fn is_package_scoped_template(template: &str, package: &str) -> bool {
    let data_data_placeholder = template.starts_with("/data/data/<pkg>/");
    let data_user_placeholder = template.starts_with("/data/user/[0-9]*/<pkg>/")
        || template.starts_with("/data/user/*/<pkg>/");
    let sdcard_placeholder = template.starts_with("/sdcard/Android/data/<pkg>/")
        || template == "/sdcard/Android/data/<pkg>";
    let data_data_concrete = template.starts_with(&format!("/data/data/{package}/"));
    let data_user_concrete = template.starts_with(&format!("/data/user/0/{package}/"));
    let sdcard_concrete = template.starts_with(&format!("/sdcard/Android/data/{package}/"))
        || template == format!("/sdcard/Android/data/{package}");
    data_data_placeholder
        || data_user_placeholder
        || sdcard_placeholder
        || data_data_concrete
        || data_user_concrete
        || sdcard_concrete
}

const fn invalid_target<T>(
    category: RuleCategory,
    reason: &'static str,
) -> Result<T, RuleParseError> {
    Err(RuleParseError::InvalidTarget { category, reason })
}
