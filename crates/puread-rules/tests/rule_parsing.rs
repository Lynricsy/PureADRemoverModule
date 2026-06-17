#![doc = "TOML 规则解析与 schema 校验测试。"]

use puread_core::model::{ProfileKind, RiskLevel, RuleAction};
use puread_rules::{
    RuleCategory, RuleParseError, RuleTarget, parse_rules_toml, parse_rules_toml_documents,
};

const VALID_FILE: &str = include_str!("../fixtures/valid/file_path.toml");
const VALID_SDK: &str = include_str!("../fixtures/valid/sdk_cache.toml");
const VALID_SQLITE: &str = include_str!("../fixtures/valid/sqlite.toml");
const VALID_COMPONENT: &str = include_str!("../fixtures/valid/component.toml");
const VALID_APPOPS: &str = include_str!("../fixtures/valid/appops.toml");
const VALID_ROM: &str = include_str!("../fixtures/valid/rom_profile.toml");
const T9_SQLITE_RULES: &str = include_str!("../../../rules/sqlite/app-ad-databases.toml");
const SOURCE_PROMPT_IS_DATA: &str = include_str!("../fixtures/valid/source_prompt_is_data.toml");
const UNKNOWN_FIELD: &str = include_str!("../fixtures/invalid/unknown_field.toml");
const FORBIDDEN_CATEGORY: &str = include_str!("../fixtures/invalid/forbidden_category.toml");
const FORBIDDEN_FIELD: &str = include_str!("../fixtures/invalid/forbidden_field.toml");
const MISSING_SOURCE: &str = include_str!("../fixtures/invalid/missing_source.toml");
const MISSING_ROLLBACK: &str = include_str!("../fixtures/invalid/missing_rollback.toml");
const MALFORMED: &str = include_str!("../fixtures/invalid/malformed.toml");
const BAD_VALUES: &str = include_str!("../fixtures/invalid/bad_values.toml");
const BAD_PACKAGE: &str = include_str!("../fixtures/invalid/bad_package.toml");
const BAD_PATH: &str = include_str!("../fixtures/invalid/bad_path.toml");
const BAD_RISK: &str = include_str!("../fixtures/invalid/bad_risk.toml");
const BAD_DEFAULT_ENABLED: &str = include_str!("../fixtures/invalid/bad_default_enabled.toml");
const BAD_ROLLBACK: &str = include_str!("../fixtures/invalid/bad_rollback.toml");

#[test]
fn valid_category_marker_file_sqlite_component_appops_rom_parse_to_typed_values() {
    // Given: one valid fixture for each allowed rule category shape.
    let fixtures = [
        VALID_FILE,
        VALID_SDK,
        VALID_SQLITE,
        VALID_COMPONENT,
        VALID_APPOPS,
        VALID_ROM,
    ];

    // When: the TOML documents cross the rules parsing boundary.
    let parsed = parse_rules_toml_documents(&fixtures);

    // Then: callers receive typed category, action, profile, risk and target values.
    let documents = match parsed {
        Ok(value) => value,
        Err(error) => panic!("valid fixtures should parse: {error}"),
    };
    let rules = documents
        .iter()
        .flat_map(puread_rules::RuleDocument::rules)
        .collect::<Vec<_>>();
    assert_eq!(rules.len(), 6);
    assert!(rules.iter().any(|rule| {
        rule.category() == RuleCategory::FilePath
            && rule.action() == RuleAction::EmptyFile
            && rule.profile() == ProfileKind::Conservative
            && rule.risk_level() == RiskLevel::Low
            && matches!(rule.target(), RuleTarget::PathTemplate(_))
    }));
    assert!(rules.iter().any(|rule| {
        rule.category() == RuleCategory::Sqlite
            && rule.action() == RuleAction::MinimalSqlite
            && matches!(rule.target(), RuleTarget::PathTemplate(_))
    }));
    assert!(rules.iter().any(|rule| {
        rule.category() == RuleCategory::Component
            && rule.action() == RuleAction::DisableComponent
            && matches!(rule.target(), RuleTarget::Component(_))
    }));
    assert!(rules.iter().any(|rule| {
        rule.category() == RuleCategory::AppOps
            && rule.action() == RuleAction::SetAppOp
            && matches!(rule.target(), RuleTarget::AppOp { .. })
    }));
    assert!(rules.iter().any(|rule| {
        rule.category() == RuleCategory::RomProfile
            && rule.action() == RuleAction::RomSetting
            && matches!(rule.target(), RuleTarget::PathTemplate(_))
    }));
}

#[test]
fn rule_parser_rejects_unknown_fields_when_schema_contains_typos() {
    // Given: a rule document with an undeclared field.
    // When: the parser applies serde schema validation.
    let error = parse_rules_toml(UNKNOWN_FIELD);

    // Then: parsing fails instead of silently accepting the field.
    assert!(matches!(error, Err(RuleParseError::Toml { .. })));
}

#[test]
fn rule_parser_accepts_t9_sqlite_document_metadata_and_flat_source_fields() {
    // Given: the T9 SQLite rule document with schema metadata and flat source fields.
    // When: the document crosses the real parser boundary.
    let document = match parse_rules_toml(T9_SQLITE_RULES) {
        Ok(value) => value,
        Err(error) => panic!("T9 SQLite rules should parse: {error}"),
    };

    // Then: all rules are typed SQLite rules that remain disabled by default.
    assert_eq!(document.rules().len(), 7);
    assert!(document.rules().iter().all(|rule| {
        rule.category() == RuleCategory::Sqlite
            && rule.profile() == ProfileKind::Sqlite
            && !rule.default_enabled()
            && matches!(rule.target(), RuleTarget::PathTemplate(_))
    }));
}

#[test]
fn rule_parser_rejects_forbidden_capabilities_when_category_or_field_is_network_scoped() {
    // Given: documents attempting to introduce out-of-scope capabilities.
    // When: category and field guards run before typed rules are returned.
    let category_error = parse_rules_toml(FORBIDDEN_CATEGORY);
    let field_error = parse_rules_toml(FORBIDDEN_FIELD);

    // Then: both forbidden surfaces are rejected explicitly.
    assert!(matches!(
        category_error,
        Err(RuleParseError::ForbiddenCategory { category }) if category == "hosts"
    ));
    assert!(matches!(
        field_error,
        Err(RuleParseError::ForbiddenField { field }) if field == "domains"
    ));
}

#[test]
fn rule_parser_rejects_missing_source_and_rollback_metadata_when_required_fields_are_absent() {
    // Given: documents missing required provenance or rollback metadata.
    // When: they are parsed through the TOML schema boundary.
    let missing_source = parse_rules_toml(MISSING_SOURCE);
    let missing_rollback = parse_rules_toml(MISSING_ROLLBACK);

    // Then: serde rejects the incomplete rule before it can become a typed rule.
    assert!(matches!(missing_source, Err(RuleParseError::Toml { .. })));
    assert!(matches!(missing_rollback, Err(RuleParseError::Toml { .. })));
}

#[test]
fn rule_parser_rejects_malformed_toml_when_input_is_not_a_document() {
    // Given: invalid TOML syntax.
    // When: parsing starts.
    let error = parse_rules_toml(MALFORMED);

    // Then: no partially parsed rules are returned.
    assert!(matches!(error, Err(RuleParseError::Toml { .. })));
}

#[test]
fn rule_parser_rejects_invalid_action_profile_package_path_risk_default_and_rollback_values() {
    // Given: fixtures with one invalid schema value class each.
    let invalid_documents = [
        BAD_VALUES,
        BAD_PACKAGE,
        BAD_PATH,
        BAD_RISK,
        BAD_DEFAULT_ENABLED,
        BAD_ROLLBACK,
    ];

    // When / Then: every invalid boundary value is rejected before typed rules are returned.
    for document in invalid_documents {
        assert!(parse_rules_toml(document).is_err());
    }
}

#[test]
fn source_metadata_prompt_injection_remains_inert_data_when_rule_is_valid() {
    // Given: source metadata containing hostile instruction-like text.
    // When: the rule is parsed.
    let document = match parse_rules_toml(SOURCE_PROMPT_IS_DATA) {
        Ok(value) => value,
        Err(error) => panic!("source metadata must remain inert data: {error}"),
    };

    // Then: the text is preserved only as metadata and does not alter typed behavior.
    let Some(rule) = document.rules().first() else {
        panic!("valid document must contain one rule");
    };
    assert_eq!(rule.action(), RuleAction::Delete);
    assert!(rule.source().line_or_pattern().contains("IGNORE PRIOR"));
}
