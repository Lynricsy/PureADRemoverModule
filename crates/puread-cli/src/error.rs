use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("ledger file does not exist: {path}")]
    MissingLedger { path: String },
    #[error("ledger path is not a file: {path}")]
    LedgerNotFile { path: String },
    #[error("root path does not exist: {path}")]
    MissingRoot { path: String },
    #[error("root path is not a directory: {path}")]
    RootNotDirectory { path: String },
    #[error("rules path does not exist: {path}")]
    MissingRules { path: String },
    #[error("rules path is not a file or directory: {path}")]
    RulesNotFileOrDirectory { path: String },
    #[error("real restore execution is not implemented; pass --dry-run")]
    RealRestoreUnsupported,
    #[error("real scan execution is not implemented; pass --dry-run")]
    RealScanUnsupported,
    #[error(transparent)]
    Ledger(#[from] puread_core::restore_ledger::LedgerError),
    #[error("failed to read rule file {path}: {source}")]
    RuleRead { path: String, source: io::Error },
    #[error("failed to parse rule file {path}: {source}")]
    RuleParse {
        path: String,
        source: puread_rules::RuleParseError,
    },
    #[error("failed to expand rule {rule_id}: {source}")]
    PathExpansion {
        rule_id: String,
        source: puread_core::path_expansion::PathExpansionError,
    },
    #[error("filesystem error at {path}: {source}")]
    Filesystem { path: String, source: io::Error },
    #[error("failed to write JSON output: {source}")]
    JsonWrite { source: serde_json::Error },
    #[error("failed to write CLI output: {source}")]
    OutputWrite { source: io::Error },
}
