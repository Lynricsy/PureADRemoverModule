#![doc = "`PureAD` CLI 二进制入口。"]
#![allow(
    unreachable_pub,
    reason = "binary-internal command modules share types through private module boundaries"
)]

use std::io::{self, Write as _};
use std::process::ExitCode;

use clap::Parser;

mod cli;
mod error;
mod json;
mod ledger;
mod lock;
mod profile;
mod profile_execute;
mod restore;
mod restore_fs;
mod rule_plan;
mod rules;
mod scan;

use cli::{Cli, Command};
use error::CliError;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _write_result = writeln!(stderr, "{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::ApplyProfile(args) => profile::run_apply_profile(&args),
        Command::DumpReport(args) => profile::run_dump_report(&args),
        Command::JsonFieldIsZero(args) => profile::run_json_field_is_zero(&args),
        Command::Ledger(command) => ledger::run_ledger(command),
        Command::ProfileReport(args) => profile::run_profile_report(&args),
        Command::ProfileRestore(args) => profile::run_profile_restore(&args),
        Command::Restore(args) => ledger::run_restore(&args),
        Command::Rules(command) => rules::run_rules(command),
        Command::Scan(args) => scan::run_scan(&args),
        Command::Status(args) => profile::run_status(&args),
    }
}
