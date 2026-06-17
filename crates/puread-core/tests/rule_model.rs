#![doc = "规则模型边界测试。"]

use puread_core::model::{
    PackageName, ProfileKind, RestoreToken, RiskLevel, RootPath, RuleAction, RuleId,
};

#[test]
fn rule_model_rejects_invalid_package_names_when_parsing_boundary_input() {
    // Given: malformed Android package names from an untrusted rule boundary.
    let invalid_names = [
        "",
        "com",
        "com..demo",
        ".com.demo",
        "com.demo.",
        "Com.demo",
        "com.demo-*",
    ];

    // When / Then: each malformed name is rejected before it can enter the model.
    for raw in invalid_names {
        assert!(PackageName::parse(raw).is_err(), "{raw} should be invalid");
    }
}

#[test]
fn rule_model_rejects_invalid_rule_id_and_path_primitives_when_constructing_values() {
    // Given: malformed identifiers and unsafe root path primitives.
    let invalid_ids = ["", " ", "ad splash", "ad/splash", "-leading"];
    let invalid_paths = [
        "",
        "relative/path",
        "/",
        "/data",
        "/sdcard",
        "/data/../system",
    ];

    // When / Then: invalid values cannot be represented by the typed model.
    for raw in invalid_ids {
        assert!(RuleId::parse(raw).is_err(), "{raw} should be invalid");
    }
    for raw in invalid_paths {
        assert!(RootPath::parse(raw).is_err(), "{raw} should be invalid");
    }
}

#[test]
fn rule_model_parses_action_profile_and_risk_as_typed_enums_when_strings_are_valid() {
    // Given: boundary strings from a rule file.
    let action = "empty_file";
    let profile = "conservative";
    let risk = "low";

    // When: they are parsed at the boundary.
    let parsed_action = RuleAction::parse(action);
    let parsed_profile = ProfileKind::parse(profile);
    let parsed_risk = RiskLevel::parse(risk);

    // Then: downstream code receives enums, not raw strings.
    assert!(matches!(parsed_action, Ok(RuleAction::EmptyFile)));
    assert!(matches!(parsed_profile, Ok(ProfileKind::Conservative)));
    assert!(matches!(parsed_risk, Ok(RiskLevel::Low)));
}

#[test]
fn rule_model_rejects_bad_action_profile_and_restore_token_strings_when_parsing_boundary_input() {
    // Given: unknown enum strings and malformed restore tokens from rule/ledger input.
    let bad_actions = ["", "rm_rf", "iptables", "disable_dns"];
    let bad_profiles = ["", "default", "global", "always_on"];
    let bad_tokens = ["", "token space", "restore/slash", "中文"];

    // When / Then: unsupported values are rejected at the parsing boundary.
    for raw in bad_actions {
        assert!(RuleAction::parse(raw).is_err(), "{raw} should be invalid");
    }
    for raw in bad_profiles {
        assert!(ProfileKind::parse(raw).is_err(), "{raw} should be invalid");
    }
    for raw in bad_tokens {
        assert!(RestoreToken::parse(raw).is_err(), "{raw} should be invalid");
    }
}

#[test]
fn rule_model_constructs_valid_values_when_all_invariants_hold() {
    // Given: a conservative file-cache rule shape with valid primitives.
    let id = RuleId::parse("coolapk-splash-cache");
    let package = PackageName::parse("com.coolapk.market");
    let root = RootPath::parse("/data/data/com.coolapk.market/cache");
    let token = RestoreToken::parse("restore-20260617-coolapk-splash");

    // When / Then: every primitive is accepted and produces doc-visible debug output.
    println!("valid_rule_fixture={id:?} {package:?} {root:?} {token:?}");
    assert!(id.is_ok());
    assert!(package.is_ok());
    assert!(root.is_ok());
    assert!(token.is_ok());
}
