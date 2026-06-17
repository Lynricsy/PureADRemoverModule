use puread_core::model::{ProfileKind, RiskLevel, RuleAction};

use crate::RuleCategory;
use crate::error::RuleParseError;

pub(crate) fn require_text(field: &'static str, value: &str) -> Result<(), RuleParseError> {
    if value.trim().is_empty() {
        return Err(RuleParseError::EmptyMetadata { field });
    }
    Ok(())
}

pub(crate) fn validate_optional_text(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), RuleParseError> {
    if let Some(raw) = value {
        require_text(field, raw)?;
    }
    Ok(())
}

pub(crate) fn validate_action(
    category: RuleCategory,
    action: RuleAction,
) -> Result<(), RuleParseError> {
    let action_name = action.as_str();
    let allowed = match category {
        RuleCategory::FilePath | RuleCategory::SdkCache => matches!(
            action_name,
            "delete" | "empty_file" | "empty_dir" | "chmod_000" | "deny_write"
        ),
        RuleCategory::Sqlite => matches!(action_name, "delete" | "minimal_sqlite" | "deny_write"),
        RuleCategory::Component => action_name == "disable_component",
        RuleCategory::AppOps => action_name == "set_appop",
        RuleCategory::RomProfile => action_name == "rom_setting",
    };
    if allowed {
        return Ok(());
    }
    Err(RuleParseError::ActionCategoryMismatch {
        category,
        action: action_name,
    })
}

pub(crate) fn validate_profile(
    category: RuleCategory,
    profile: ProfileKind,
) -> Result<(), RuleParseError> {
    let profile_name = profile.as_str();
    let allowed = matches!(
        (category, profile_name),
        (RuleCategory::FilePath, "conservative")
            | (RuleCategory::SdkCache, "sdk_cache")
            | (RuleCategory::Sqlite, "sqlite")
            | (RuleCategory::Component, "component")
            | (RuleCategory::AppOps, "appops")
            | (RuleCategory::RomProfile, "rom")
    );
    if allowed {
        return Ok(());
    }
    Err(RuleParseError::ProfileCategoryMismatch {
        category,
        profile: profile_name,
    })
}

pub(crate) fn validate_default_enabled(
    category: RuleCategory,
    risk: RiskLevel,
    default_enabled: bool,
) -> Result<(), RuleParseError> {
    let must_be_disabled = risk.as_str() == "high"
        || matches!(
            category,
            RuleCategory::Sqlite
                | RuleCategory::Component
                | RuleCategory::AppOps
                | RuleCategory::RomProfile
        );
    if !must_be_disabled || !default_enabled {
        return Ok(());
    }
    Err(RuleParseError::DefaultEnabledMismatch {
        category,
        default_enabled,
    })
}
