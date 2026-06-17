use crate::error::RuleParseError;
use crate::raw::RawDocument;
use crate::rule::RuleDocument;

const FORBIDDEN_FIELDS: &[&str] = &[
    "hosts",
    "host",
    "dns",
    "private_dns",
    "domain",
    "domains",
    "proxy",
    "clash",
    "mihomo",
    "adguardhome",
    "iptables",
    "mount_hosts",
    "ad_reward",
    "ifw_clear",
    "zygisk",
    "root_hide",
];

/// 解析单个 TOML 规则文档。
pub fn parse_rules_toml(input: &str) -> Result<RuleDocument, RuleParseError> {
    reject_forbidden_fields(input)?;
    let raw = toml::from_str::<RawDocument>(input)?;
    RuleDocument::from_raw(raw)
}

/// 解析多个 TOML 规则文档。
pub fn parse_rules_toml_documents(inputs: &[&str]) -> Result<Vec<RuleDocument>, RuleParseError> {
    inputs.iter().map(|input| parse_rules_toml(input)).collect()
}

fn reject_forbidden_fields(input: &str) -> Result<(), RuleParseError> {
    let value = toml::from_str::<toml::Value>(input)?;
    scan_value_keys(&value)
}

fn scan_value_keys(value: &toml::Value) -> Result<(), RuleParseError> {
    match value {
        toml::Value::Table(table) => {
            for (field, nested) in table {
                if is_forbidden_field(field) {
                    return Err(RuleParseError::ForbiddenField {
                        field: field.to_owned(),
                    });
                }
                scan_value_keys(nested)?;
            }
            Ok(())
        }
        toml::Value::Array(values) => {
            for nested in values {
                scan_value_keys(nested)?;
            }
            Ok(())
        }
        toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => Ok(()),
    }
}

fn is_forbidden_field(field: &str) -> bool {
    FORBIDDEN_FIELDS
        .iter()
        .any(|forbidden| field.eq_ignore_ascii_case(forbidden))
}
