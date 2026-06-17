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
    Ledger(LedgerCommand),
    Restore(RestoreArgs),
    Rules(RulesCommand),
    Scan(ScanArgs),
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
    pub ledger: PathBuf,
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
    pub rules: PathBuf,
    #[arg(long)]
    pub root: PathBuf,
}
