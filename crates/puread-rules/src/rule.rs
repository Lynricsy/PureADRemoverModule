mod metadata;
mod rollback_input;
mod schedule;
mod source_input;
mod target_input;

use puread_core::model::{PackageName, ProfileKind, RiskLevel, RuleAction, RuleId};

use crate::RuleCategory;
use crate::error::RuleParseError;
use crate::raw::{RawDocument, RawRule};
use crate::rollback::RollbackStrategy;
use crate::source::RuleSource;
use crate::target::{RuleTarget, build_target};
use crate::validation::{
    require_text, validate_action, validate_default_enabled, validate_profile,
};

use self::metadata::validate_document_metadata;
use self::rollback_input::rollback_strategy_from_raw;
use self::schedule::validate_schedule;
use self::source_input::source_from_raw;
use self::target_input::{normalized_rule, package_from_raw};

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
