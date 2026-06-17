use puread_core::model::{PackageName, ProfileKind, RiskLevel, RuleAction, RuleId};

use crate::RuleCategory;
use crate::error::RuleParseError;
use crate::raw::{RawDocument, RawRule, RawSource, RawSourceInput};
use crate::rollback::RollbackStrategy;
use crate::source::RuleSource;
use crate::target::{RuleTarget, build_target};
use crate::validation::{
    require_text, validate_action, validate_default_enabled, validate_profile,
};

/// 一个 TOML 文件解析后的类型化规则文档。
#[derive(Debug, Clone)]
pub struct RuleDocument {
    rules: Vec<RuleDefinition>,
}

impl RuleDocument {
    pub(crate) fn from_raw(raw: RawDocument) -> Result<Self, RuleParseError> {
        validate_document_metadata(&raw)?;
        let rules = raw
            .rules
            .into_iter()
            .map(RuleDefinition::from_raw)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { rules })
    }

    /// 返回文档中的规则。
    pub fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }
}

/// 单条已通过 schema 校验的规则。
#[derive(Debug, Clone)]
pub struct RuleDefinition {
    id: RuleId,
    category: RuleCategory,
    package: PackageName,
    action: RuleAction,
    target: RuleTarget,
    risk_level: RiskLevel,
    default_enabled: bool,
    profile: ProfileKind,
    observed_behavior: String,
    rollback_strategy: RollbackStrategy,
    introduced_by: String,
    reviewed_at: String,
    notes: Option<String>,
    source: RuleSource,
}

impl RuleDefinition {
    fn from_raw(raw: RawRule) -> Result<Self, RuleParseError> {
        let id = RuleId::parse(&raw.id)?;
        let category = RuleCategory::parse(&raw.category)?;
        let package = package_from_raw(&raw, category)?;
        let action = RuleAction::parse(&raw.action)?;
        let risk_level = RiskLevel::parse(&raw.risk_level)?;
        let profile = ProfileKind::parse(&raw.profile)?;
        validate_action(category, action)?;
        validate_schedule(category, raw.schedule.as_deref())?;
        validate_profile(category, profile)?;
        validate_default_enabled(category, risk_level, raw.default_enabled)?;
        require_text("observed_behavior", &raw.observed_behavior)?;
        require_text("introduced_by", &raw.introduced_by)?;
        require_text("reviewed_at", &raw.reviewed_at)?;
        let normalized_target = normalized_rule(&raw, category, package.as_str())?;
        let target = build_target(category, &package, &normalized_target)?;
        let rollback_strategy = rollback_strategy_from_raw(category, &raw.rollback_strategy)?;
        let source = RuleSource::from_raw(source_from_raw(raw.source.clone(), &raw)?)?;
        Ok(Self {
            id,
            category,
            package,
            action,
            target,
            risk_level,
            default_enabled: raw.default_enabled,
            profile,
            observed_behavior: raw.observed_behavior,
            rollback_strategy,
            introduced_by: raw.introduced_by,
            reviewed_at: raw.reviewed_at,
            notes: raw.notes,
            source,
        })
    }

    /// 返回规则 ID。
    pub const fn id(&self) -> &RuleId {
        &self.id
    }

    /// 返回规则类别。
    pub const fn category(&self) -> RuleCategory {
        self.category
    }

    /// 返回目标包名。
    pub const fn package(&self) -> &PackageName {
        &self.package
    }

    /// 返回规则动作。
    pub const fn action(&self) -> RuleAction {
        self.action
    }

    /// 返回规则目标。
    pub const fn target(&self) -> &RuleTarget {
        &self.target
    }

    /// 返回风险等级。
    pub const fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }

    /// 返回默认启用状态。
    pub const fn default_enabled(&self) -> bool {
        self.default_enabled
    }

    /// 返回启用画像。
    pub const fn profile(&self) -> ProfileKind {
        self.profile
    }

    /// 返回上游行为说明。
    pub fn observed_behavior(&self) -> &str {
        &self.observed_behavior
    }

    /// 返回恢复策略。
    pub const fn rollback_strategy(&self) -> RollbackStrategy {
        self.rollback_strategy
    }

    /// 返回引入任务。
    pub fn introduced_by(&self) -> &str {
        &self.introduced_by
    }

    /// 返回审查日期。
    pub fn reviewed_at(&self) -> &str {
        &self.reviewed_at
    }

    /// 返回可选备注。
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    /// 返回来源元数据。
    pub const fn source(&self) -> &RuleSource {
        &self.source
    }
}

fn validate_document_metadata(raw: &RawDocument) -> Result<(), RuleParseError> {
    if let Some(version) = raw.schema_version
        && version != 1
    {
        return Err(RuleParseError::UnsupportedDocumentMetadata {
            field: "schema_version",
            value: version.to_string(),
        });
    }
    if let Some(kind) = raw.kind.as_deref()
        && kind != "sqlite"
    {
        return Err(RuleParseError::UnsupportedDocumentMetadata {
            field: "kind",
            value: kind.to_owned(),
        });
    }
    Ok(())
}

fn package_from_raw(raw: &RawRule, category: RuleCategory) -> Result<PackageName, RuleParseError> {
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

fn normalized_rule(
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
        return Err(RuleParseError::InvalidTarget {
            category: RuleCategory::Sqlite,
            reason: "sqlite target must stay under /data/user",
        });
    };
    let Some((user_segment, after_user_id)) = after_user.split_once('/') else {
        return Err(RuleParseError::InvalidTarget {
            category: RuleCategory::Sqlite,
            reason: "sqlite target must include user id segment",
        });
    };
    let Some((package_segment, suffix)) = after_user_id.split_once('/') else {
        return Err(RuleParseError::InvalidTarget {
            category: RuleCategory::Sqlite,
            reason: "sqlite target must include package segment",
        });
    };
    if package_segment != package && !(package_segment == "*" && package == "puread.sqlite.any") {
        return Err(RuleParseError::InvalidTarget {
            category: RuleCategory::Sqlite,
            reason: "sqlite package segment mismatch",
        });
    }
    Ok(format!("/data/user/{user_segment}/<pkg>/{suffix}"))
}

fn sqlite_package_from_target(raw: &RawRule) -> Result<&str, RuleParseError> {
    let Some(template) = raw.target_template.as_deref() else {
        return Err(RuleParseError::InvalidTarget {
            category: RuleCategory::Sqlite,
            reason: "sqlite rule requires target_template",
        });
    };
    let Some(after_user) = template.strip_prefix("/data/user/") else {
        return Err(RuleParseError::InvalidTarget {
            category: RuleCategory::Sqlite,
            reason: "sqlite target must stay under /data/user",
        });
    };
    let Some((_, after_user_id)) = after_user.split_once('/') else {
        return Err(RuleParseError::InvalidTarget {
            category: RuleCategory::Sqlite,
            reason: "sqlite target must include user id segment",
        });
    };
    let Some((package, _)) = after_user_id.split_once('/') else {
        return Err(RuleParseError::InvalidTarget {
            category: RuleCategory::Sqlite,
            reason: "sqlite target must include package segment",
        });
    };
    if package == "*" {
        return Ok("puread.sqlite.any");
    }
    Ok(package)
}

fn rollback_strategy_from_raw(
    category: RuleCategory,
    raw: &str,
) -> Result<RollbackStrategy, RuleParseError> {
    match RollbackStrategy::parse(raw) {
        Ok(strategy) => Ok(strategy),
        Err(error) if category == RuleCategory::Sqlite => sqlite_rollback_strategy(raw, error),
        Err(error) => Err(error),
    }
}

fn sqlite_rollback_strategy(
    raw: &str,
    error: RuleParseError,
) -> Result<RollbackStrategy, RuleParseError> {
    if raw.contains("snapshot the original database path") && raw.contains("restore") {
        return Ok(RollbackStrategy::RestoreOriginal);
    }
    Err(error)
}

fn source_from_raw(source: RawSourceInput, raw: &RawRule) -> Result<RawSource, RuleParseError> {
    match source {
        RawSourceInput::Nested(nested) => {
            if raw.source_file.is_some()
                || raw.zip_entry.is_some()
                || raw.source_line_or_pattern.is_some()
            {
                return Err(RuleParseError::InvalidSourceMetadata {
                    reason: "nested source must not be mixed with flat source fields",
                });
            }
            Ok(nested)
        }
        RawSourceInput::Name(source) => {
            let Some(source_line_or_pattern) = raw.source_line_or_pattern.clone() else {
                return Err(RuleParseError::InvalidSourceMetadata {
                    reason: "flat source requires source_line_or_pattern",
                });
            };
            Ok(RawSource {
                source,
                source_file: raw.source_file.clone(),
                zip_entry: raw.zip_entry.clone(),
                source_line_or_pattern,
            })
        }
    }
}

fn validate_schedule(category: RuleCategory, schedule: Option<&str>) -> Result<(), RuleParseError> {
    let Some(schedule) = schedule else {
        return Ok(());
    };
    if category != RuleCategory::Sqlite {
        return Err(RuleParseError::InvalidTarget {
            category,
            reason: "only sqlite rules may define schedule",
        });
    }
    match schedule {
        "manual" | "boot_once" | "low_frequency" => Ok(()),
        _ => Err(RuleParseError::InvalidTarget {
            category,
            reason: "unsupported sqlite schedule",
        }),
    }
}
