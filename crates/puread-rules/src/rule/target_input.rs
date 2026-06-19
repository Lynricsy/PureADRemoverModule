use puread_core::model::PackageName;

use crate::RuleCategory;
use crate::error::RuleParseError;
use crate::raw::RawRule;

pub(super) fn package_from_raw(
    raw: &RawRule,
    category: RuleCategory,
) -> Result<PackageName, RuleParseError> {
    if let Some(package) = raw.package.as_deref() {
        return Ok(PackageName::parse(package)?);
    }
    if category == RuleCategory::Sqlite {
        return Ok(PackageName::parse(sqlite_package_from_target(raw)?)?);
    }
    Err(RuleParseError::InvalidTarget {
        category,
        reason: "rule requires package",
    })
}

pub(super) fn normalized_rule(
    raw: &RawRule,
    category: RuleCategory,
    package: &str,
) -> Result<RawRule, RuleParseError> {
    if category != RuleCategory::Sqlite {
        return Ok(raw.clone());
    }
    let Some(template) = raw.target_template.as_deref() else {
        return Ok(raw.clone());
    };
    if template.contains("<pkg>") {
        return Ok(raw.clone());
    }
    let normalized_template = normalize_sqlite_template(template, package)?;
    let mut normalized = raw.clone();
    normalized.target_template = Some(normalized_template);
    Ok(normalized)
}

fn normalize_sqlite_template(template: &str, package: &str) -> Result<String, RuleParseError> {
    let Some(after_user) = template.strip_prefix("/data/user/") else {
        return sqlite_target_error("sqlite target must stay under /data/user");
    };
    let Some((user_segment, after_user_id)) = after_user.split_once('/') else {
        return sqlite_target_error("sqlite target must include user id segment");
    };
    let Some((package_segment, suffix)) = after_user_id.split_once('/') else {
        return sqlite_target_error("sqlite target must include package segment");
    };
    if package_segment == "*" && package == "puread.sqlite.any" {
        return Ok(template.to_owned());
    }
    if package_segment != package {
        return sqlite_target_error("sqlite package segment mismatch");
    }
    Ok(format!("/data/user/{user_segment}/<pkg>/{suffix}"))
}

fn sqlite_package_from_target(raw: &RawRule) -> Result<&str, RuleParseError> {
    let Some(template) = raw.target_template.as_deref() else {
        return sqlite_target_error("sqlite rule requires target_template");
    };
    let Some(after_user) = template.strip_prefix("/data/user/") else {
        return sqlite_target_error("sqlite target must stay under /data/user");
    };
    let Some((_, after_user_id)) = after_user.split_once('/') else {
        return sqlite_target_error("sqlite target must include user id segment");
    };
    let Some((package, _)) = after_user_id.split_once('/') else {
        return sqlite_target_error("sqlite target must include package segment");
    };
    if package == "*" {
        return Ok("puread.sqlite.any");
    }
    Ok(package)
}

const fn sqlite_target_error<T>(reason: &'static str) -> Result<T, RuleParseError> {
    Err(RuleParseError::InvalidTarget {
        category: RuleCategory::Sqlite,
        reason,
    })
}
