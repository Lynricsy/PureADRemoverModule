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
        Command::Ledger(command) => ledger::run_ledger(command),
        Command::Restore(args) => ledger::run_restore(&args),
        Command::Rules(command) => rules::run_rules(command),
        Command::Scan(args) => scan::run_scan(&args),
    }
}
