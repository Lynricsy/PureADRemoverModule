#![doc = "`PureAD` daemon 二进制入口的最小脚手架。"]

use std::process::ExitCode;

/// Daemon 二进制入口。
pub const fn main() -> ExitCode {
    ExitCode::SUCCESS
}
