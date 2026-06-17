use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDocument {
    pub(crate) schema_version: Option<u8>,
    pub(crate) kind: Option<String>,
    pub(crate) rules: Vec<RawRule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawRule {
    pub(crate) id: String,
    pub(crate) category: String,
    pub(crate) package: Option<String>,
    pub(crate) action: String,
    pub(crate) schedule: Option<String>,
    pub(crate) target_template: Option<String>,
    pub(crate) target_component: Option<String>,
    pub(crate) appop: Option<String>,
    pub(crate) appop_mode: Option<String>,
    pub(crate) risk_level: String,
    pub(crate) default_enabled: bool,
    pub(crate) profile: String,
    pub(crate) observed_behavior: String,
    pub(crate) rollback_strategy: String,
    pub(crate) introduced_by: String,
    pub(crate) reviewed_at: String,
    pub(crate) notes: Option<String>,
    pub(crate) source: RawSourceInput,
    pub(crate) source_file: Option<String>,
    pub(crate) zip_entry: Option<String>,
    pub(crate) source_line_or_pattern: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawSourceInput {
    Nested(RawSource),
    Name(String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSource {
    pub(crate) source: String,
    pub(crate) source_file: Option<String>,
    pub(crate) zip_entry: Option<String>,
    pub(crate) source_line_or_pattern: String,
}
