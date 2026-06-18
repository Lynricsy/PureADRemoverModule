//! 可注入的 Android 命令适配层。

mod adapter;
mod appops;
mod error;
mod invocation;
mod metadata;
mod output;
mod pm;
mod property;
mod runner;
mod settings;
mod validation;

pub use adapter::{AndroidCommandAdapter, CommandOutcome, CommandPhase};
pub use appops::AppOpsAdapter;
pub use error::CommandError;
pub use invocation::CommandInvocation;
pub use metadata::{ChattrAdapter, ChconAdapter, LsattrAdapter};
pub use output::CommandOutput;
pub use pm::PmComponentAdapter;
pub use property::GetpropAdapter;
pub use runner::{AndroidCommandRunner, CommandRunnerError, RealAndroidCommandRunner};
pub use settings::{SettingsAdapter, SettingsNamespace};
