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
    #[error("--dry-run and --execute cannot be used together")]
    ConflictingExecutionMode,
    #[error("lock path has no parent directory: {path}")]
    LockPathHasNoParent { path: String },
    #[error("global lock is already held: {path}")]
    LockAlreadyHeld { path: String },
    #[error("profile action is not supported by CLI file executor: {action}")]
    UnsupportedProfileAction { action: &'static str },
    #[error("profile ledger JSON failed at {path}: {source}")]
    ProfileLedgerJson {
        path: String,
        source: serde_json::Error,
    },
    #[error("profile ledger record is missing kind: {path}")]
    ProfileLedgerMissingKind { path: String },
    #[error("profile restore failed for {failed} action(s)")]
    ProfileRestoreFailed { failed: usize },
    #[error(transparent)]
    Model(#[from] puread_core::error::ModelError),
    #[error("invalid action target {path}: {reason}")]
    InvalidActionTarget { path: String, reason: &'static str },
    #[error("restore ledger path cannot be mapped to fixture root: {path}")]
    RestorePathOutOfRoot { path: String },
    #[error("file action failed: {source}")]
    FileAction {
        #[from]
        source: puread_android::file_actions::FileActionError,
    },
    #[error("sqlite action failed: {source}")]
    SqliteAction {
        #[from]
        source: puread_android::sqlite_actions::SqliteActionError,
    },
    #[error("android command profile failed: {source}")]
    AndroidCommand {
        #[from]
        source: puread_android::command_runner::CommandError,
    },
    #[error("android profile failed: {source}")]
    AndroidProfile {
        #[from]
        source: puread_android::profiles::ProfileError,
    },
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
    #[error("JSON field {field} in {path} is not zero")]
    JsonFieldNotZero { path: String, field: String },
    #[error("failed to write JSON output: {source}")]
    JsonWrite { source: serde_json::Error },
    #[error("failed to write CLI output: {source}")]
    OutputWrite { source: io::Error },
}
