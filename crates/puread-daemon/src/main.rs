#![doc = "`PureAD` daemon 二进制入口。"]

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use puread_daemon::{
    DaemonError, DaemonEvent, EventLoop, FileRuleDaemonConfig, FileRuleDaemonMode,
    FileRuleDaemonRuntime, SignalForwarder,
};

const DEFAULT_ROOT: &str = "/";
const DEFAULT_RULES: &str = "/data/adb/modules/PureAD/rules";
const DEFAULT_STATE: &str = "/data/adb/modules/PureAD/state";
const DEFAULT_DEBOUNCE_MS: u64 = 1_000;

/// 运行 `PureAD` daemon 二进制。
pub fn main() -> ExitCode {
    match entrypoint() {
        Ok(()) | Err(MainError::Args(CliArgError::HelpRequested)) => ExitCode::SUCCESS,
        Err(error) => {
            drop(writeln!(std::io::stderr(), "puread-daemon: {error}"));
            ExitCode::FAILURE
        }
    }
}

fn entrypoint() -> Result<(), MainError> {
    run_daemon(&DaemonArgs::parse()?)?;
    Ok(())
}

fn run_daemon(args: &DaemonArgs) -> Result<(), DaemonError> {
    let config = FileRuleDaemonConfig::new(
        args.root.clone(),
        vec![args.rules.clone()],
        args.mode.clone(),
        args.debounce,
    )?;
    let runtime = config.prepare()?;
    let mut logger = EventLogger::open(args.log_file.as_deref())?;
    logger.event(&DaemonEvent::Started)?;
    logger.runtime(&runtime)?;
    if runtime.watch_roots().is_empty() {
        logger.line("no_watch_roots=true")?;
        return Ok(());
    }
    let (mut event_loop, handle) = EventLoop::from_file_rule_runtime(runtime)?;
    let _signals = SignalForwarder::start(handle)?;
    event_loop.run(|event| logger.event(&event))
}

#[derive(Debug, thiserror::Error)]
enum MainError {
    #[error(transparent)]
    Args(#[from] CliArgError),
    #[error(transparent)]
    Daemon(#[from] puread_daemon::DaemonError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DaemonArgs {
    root: PathBuf,
    rules: PathBuf,
    mode: FileRuleDaemonMode,
    debounce: Duration,
    log_file: Option<PathBuf>,
}

impl DaemonArgs {
    fn parse() -> Result<Self, CliArgError> {
        Self::parse_from(std::env::args().skip(1))
    }

    fn parse_from(args: impl IntoIterator<Item = String>) -> Result<Self, CliArgError> {
        let mut builder = DaemonArgsBuilder::default();
        let mut items = args.into_iter();
        while let Some(arg) = items.next() {
            match arg.as_str() {
                "--dry-run" => builder.apply_mode(ModeArg::DryRun)?,
                "--apply" => builder.apply_mode(ModeArg::Apply)?,
                "--root" => builder.root = Some(next_value(&mut items, "--root")?),
                "--rules" => builder.rules = Some(next_value(&mut items, "--rules")?),
                "--state-dir" => builder.state_dir = Some(next_value(&mut items, "--state-dir")?),
                "--ledger" => builder.ledger = Some(next_value(&mut items, "--ledger")?),
                "--log-file" => builder.log_file = Some(next_value(&mut items, "--log-file")?),
                "--debounce-ms" => {
                    builder.debounce_ms =
                        Some(parse_u64(next_value(&mut items, "--debounce-ms")?)?);
                }
                "--help" | "-h" => return Err(CliArgError::HelpRequested),
                unknown => return Err(CliArgError::UnknownFlag(unknown.to_owned())),
            }
        }
        builder.build()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModeArg {
    DryRun,
    Apply,
}

#[derive(Debug, Default)]
struct DaemonArgsBuilder {
    root: Option<String>,
    rules: Option<String>,
    state_dir: Option<String>,
    ledger: Option<String>,
    mode: Option<ModeArg>,
    debounce_ms: Option<u64>,
    log_file: Option<String>,
}

impl DaemonArgsBuilder {
    const fn apply_mode(&mut self, mode: ModeArg) -> Result<(), CliArgError> {
        if self.mode.is_some() {
            return Err(CliArgError::ConflictingMode);
        }
        self.mode = Some(mode);
        Ok(())
    }

    fn build(self) -> Result<DaemonArgs, CliArgError> {
        let root = path_or_default(self.root, DEFAULT_ROOT);
        let rules = path_or_default(self.rules, DEFAULT_RULES);
        let state_dir = path_or_default(self.state_dir, DEFAULT_STATE);
        let ledger = self
            .ledger
            .map_or_else(|| state_dir.join("actions.jsonl"), PathBuf::from);
        let mode = match self.mode.unwrap_or(ModeArg::DryRun) {
            ModeArg::DryRun => FileRuleDaemonMode::DryRun,
            ModeArg::Apply => FileRuleDaemonMode::Apply {
                ledger_path: ledger,
            },
        };
        let debounce_ms = self.debounce_ms.unwrap_or(DEFAULT_DEBOUNCE_MS);
        if debounce_ms == 0 {
            return Err(CliArgError::InvalidDebounce);
        }
        Ok(DaemonArgs {
            root,
            rules,
            mode,
            debounce: Duration::from_millis(debounce_ms),
            log_file: self.log_file.map(PathBuf::from),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
enum CliArgError {
    #[error("help requested")]
    HelpRequested,
    #[error("unknown daemon flag: {0}")]
    UnknownFlag(String),
    #[error("missing value for daemon flag: {0}")]
    MissingValue(&'static str),
    #[error("--apply and --dry-run cannot be used together")]
    ConflictingMode,
    #[error("debounce must be greater than zero milliseconds")]
    InvalidDebounce,
    #[error("invalid integer value: {0}")]
    InvalidInteger(String),
}

struct EventLogger {
    file: Option<File>,
}

impl EventLogger {
    fn open(path: Option<&Path>) -> Result<Self, DaemonError> {
        let Some(path) = path else {
            return Ok(Self { file: None });
        };
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|source| DaemonError::LogOpen {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(Self { file: Some(file) })
    }

    fn runtime(&mut self, runtime: &FileRuleDaemonRuntime) -> Result<(), DaemonError> {
        self.line(&format!("file_rule_count={}", runtime.file_rule_count()))?;
        self.line(&format!(
            "skipped_high_frequency_rule_count={}",
            runtime.skipped_high_frequency_rule_count()
        ))?;
        self.line(&format!("watch_root_count={}", runtime.watch_roots().len()))
    }

    fn event(&mut self, event: &DaemonEvent) -> Result<(), DaemonError> {
        match event {
            DaemonEvent::Started => self.line("event=started"),
            DaemonEvent::ReloadRequested => self.line("event=reload_requested"),
            DaemonEvent::ShutdownRequested => self.line("event=shutdown_requested"),
            DaemonEvent::FilesChanged { paths } => {
                self.line(&format!("event=files_changed count={}", paths.len()))
            }
            DaemonEvent::DryRunFilePlan { actions } => {
                self.line(&format!("event=dry_run_file_plan count={}", actions.len()))
            }
            DaemonEvent::FileRuleApplyReport { outcomes } => self.line(&format!(
                "event=file_rule_apply_report count={}",
                outcomes.len()
            )),
            _ => self.line("event=unknown"),
        }
    }

    fn line(&mut self, message: &str) -> Result<(), DaemonError> {
        let Some(file) = &mut self.file else {
            return Ok(());
        };
        writeln!(file, "{message}").map_err(|source| DaemonError::LogWrite { source })
    }
}

fn path_or_default(value: Option<String>, default: &str) -> PathBuf {
    value.map_or_else(|| PathBuf::from(default), PathBuf::from)
}

fn next_value(
    args: &mut impl Iterator<Item = String>,
    flag: &'static str,
) -> Result<String, CliArgError> {
    args.next().ok_or(CliArgError::MissingValue(flag))
}

fn parse_u64(value: String) -> Result<u64, CliArgError> {
    value
        .parse::<u64>()
        .map_err(|_source| CliArgError::InvalidInteger(value))
}

#[cfg(test)]
mod main_tests;
