#![doc = "参数错误行为测试。"]

use std::error::Error;
use std::process::{Command, Output};

#[test]
fn report_fails_cleanly_when_argument_is_unknown() -> Result<(), Box<dyn Error>> {
    // Given: an unsupported argument.
    // When: the report-only tool is invoked through clap.
    let output = run_report(["--not-a-real-flag"])?;

    // Then: it exits non-zero and reports the unknown argument.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("unexpected argument"), "{stderr}");
    Ok(())
}

fn run_report<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-upstream-report"))
        .args(args)
        .output()?)
}
