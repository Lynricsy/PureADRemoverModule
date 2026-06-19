use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "puread-cli", version, about = "PureAD 本地层治理 CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    ApplyProfile(ApplyProfileArgs),
    DumpReport(DumpReportArgs),
    JsonFieldIsZero(JsonFieldIsZeroArgs),
    Ledger(LedgerCommand),
    ProfileReport(ProfileReportArgs),
    ProfileRestore(ProfileRestoreArgs),
    Restore(RestoreArgs),
    Rules(RulesCommand),
    Scan(ScanArgs),
    Status(StatusArgs),
}

#[derive(Debug, Args)]
pub struct CommonPathArgs {
    #[arg(long, default_value = "rules/files")]
    pub rules: PathBuf,
    #[arg(long, default_value = "/")]
    pub root: PathBuf,
    #[arg(long, default_value = "/data/adb/modules/PureAD")]
    pub module_root: PathBuf,
    #[arg(long)]
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[command(flatten)]
    pub paths: CommonPathArgs,
}

#[derive(Debug, Args)]
pub struct ApplyProfileArgs {
    pub profile: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub execute: bool,
    #[cfg(debug_assertions)]
    #[command(flatten)]
    pub profile_test: ApplyProfileTestArgs,
    #[command(flatten)]
    pub paths: CommonPathArgs,
}

#[cfg(debug_assertions)]
#[derive(Debug, Args)]
pub struct ApplyProfileTestArgs {
    #[arg(long, hide = true)]
    pub test_profile_runner: bool,
    #[arg(long, hide = true)]
    pub profile_runner_log: Option<PathBuf>,
    #[arg(long, hide = true)]
    pub test_profile_ledger_fail: bool,
}

#[derive(Debug, Args)]
pub struct DumpReportArgs {
    #[arg(long)]
    pub ledger: PathBuf,
}

#[derive(Debug, Args)]
pub struct JsonFieldIsZeroArgs {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub field: String,
}

#[derive(Debug, Args)]
pub struct LedgerCommand {
    #[command(subcommand)]
    pub command: LedgerSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum LedgerSubcommand {
    Show(LedgerPathArgs),
}

#[derive(Debug, Args)]
pub struct LedgerPathArgs {
    #[arg(long)]
    pub ledger: PathBuf,
}

#[derive(Debug, Args)]
pub struct RestoreArgs {
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub execute: bool,
    #[arg(long)]
    pub ledger: PathBuf,
}

#[derive(Debug, Args)]
pub struct ProfilePathArgs {
    #[arg(long, default_value = "/data/adb/modules/PureAD")]
    pub module_root: PathBuf,
    #[arg(long)]
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ProfileReportArgs {
    #[command(flatten)]
    pub paths: ProfilePathArgs,
    #[arg(long, value_enum, default_value_t = ReportFormat::Json)]
    pub format: ReportFormat,
}

#[derive(Debug, Args)]
pub struct ProfileRestoreArgs {
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub execute: bool,
    #[command(flatten)]
    pub paths: ProfilePathArgs,
    #[arg(long, value_enum, default_value_t = ReportFormat::Json)]
    pub format: ReportFormat,
    #[cfg(debug_assertions)]
    #[arg(long, hide = true)]
    pub test_profile_runner: bool,
    #[cfg(debug_assertions)]
    #[arg(long, hide = true)]
    pub profile_runner_log: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ReportFormat {
    Json,
    Text,
}

#[derive(Debug, Args)]
pub struct RulesCommand {
    #[command(subcommand)]
    pub command: RulesSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum RulesSubcommand {
    Validate(RulesValidateArgs),
    List(RulesListArgs),
}

#[derive(Debug, Args)]
pub struct RulesValidateArgs {
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub struct RulesListArgs {
    #[arg(long, value_enum)]
    pub kind: RulesListKind,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RulesListKind {
    Files,
    Sqlite,
}

#[derive(Debug, Args)]
pub struct ScanArgs {
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub execute: bool,
    #[arg(long, default_value = "rules/files")]
    pub rules: PathBuf,
    #[arg(long)]
    pub root: PathBuf,
}
