#![doc = "`PureAD` 上游 report-only 工具入口。"]
#![allow(
    unreachable_pub,
    reason = "binary-internal modules share typed report structures across private module boundaries"
)]

use std::io::{self, Write as _};
use std::process::ExitCode;

use clap::Parser as _;

mod classifier;
mod cli;
mod error;
mod manifest;
mod report;
mod scanner;

use cli::Cli;
use error::ReportError;

fn main() -> ExitCode {
    let args = match Cli::try_parse() {
        Ok(args) => args,
        Err(error) => {
            let _print_result = error.print();
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _write_result = writeln!(stderr, "error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Cli) -> Result<(), ReportError> {
    let result = scanner::scan(args)?;
    let rendered = report::render(&result);
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;
    Ok(())
}
