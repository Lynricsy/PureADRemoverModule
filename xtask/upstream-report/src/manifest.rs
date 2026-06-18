use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    AdRewardDomain,
    Appops,
    Component,
    Dns,
    Domain,
    FilePath,
    Hosts,
    IfwClear,
    IptablesNetwork,
    Proxy,
    RomProfile,
    SdkCache,
    Sqlite,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Directory,
    Zip,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    File,
    ZipEntry,
}

#[derive(Debug, Serialize)]
pub struct Manifest {
    pub schema_version: u8,
    pub generated_at: String,
    pub mode: &'static str,
    pub input: InputSummary,
    pub policy: Policy,
    pub summary: Summary,
    pub sources: Vec<SourceRecord>,
    pub accepted: Vec<FindingRecord>,
    pub rejected: Vec<FindingRecord>,
    pub ignored: Vec<FindingRecord>,
}

#[derive(Debug, Serialize)]
pub struct InputSummary {
    pub path: String,
    pub kind: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Policy {
    pub download_performed: DisabledFlag,
    pub rules_modified: DisabledFlag,
    pub snapshots_modified: DisabledFlag,
    pub report_only: EnabledFlag,
    pub auto_import_allowed: DisabledFlag,
}

#[derive(Debug, Clone, Copy)]
pub enum DisabledFlag {
    False,
}

#[derive(Debug, Clone, Copy)]
pub enum EnabledFlag {
    True,
}

impl Serialize for DisabledFlag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(false)
    }
}

impl Serialize for EnabledFlag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(true)
    }
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub sources: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub ignored: usize,
}

#[derive(Debug, Serialize)]
pub struct SourceRecord {
    pub source_id: String,
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub file_count: usize,
    pub remote_url: String,
    pub commit: String,
    pub executable_upstream_code: bool,
    pub auto_import_allowed: bool,
}

#[derive(Debug, Serialize)]
pub struct FindingRecord {
    pub source: String,
    pub kind: ItemKind,
    pub path: String,
    pub zip_entry: Option<String>,
    pub sha256: String,
    pub size_bytes: u64,
    pub categories: Vec<Category>,
    pub signals: Vec<String>,
    pub reason: String,
    pub review_state: &'static str,
    pub auto_import_allowed: bool,
    pub executable_upstream_code: bool,
}

#[derive(Debug)]
pub struct ScanResult {
    pub manifest: Manifest,
    pub legacy_dry_run: bool,
    pub manifest_path: String,
}
