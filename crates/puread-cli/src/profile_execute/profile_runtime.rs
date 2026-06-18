use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use puread_android::command_runner::{
    AndroidCommandRunner, CommandInvocation, CommandOutput, CommandRunnerError,
    RealAndroidCommandRunner,
};
use puread_android::profiles::{ProfileError, ProfileLedgerSink};

use crate::profile_execute::PROFILE_LEDGER_RELATIVE_PATH;

const O_NOFOLLOW: i32 = 0o400_000;

#[derive(Debug, Clone)]
pub(in crate::profile_execute) enum SelectedProfileRunner {
    Real(RealAndroidCommandRunner),
    #[cfg(debug_assertions)]
    Scripted(ScriptedProfileRunner),
}

impl SelectedProfileRunner {
    pub(in crate::profile_execute) const fn real() -> Self {
        Self::Real(RealAndroidCommandRunner)
    }

    #[cfg(debug_assertions)]
    pub(in crate::profile_execute) const fn scripted(log_path: Option<PathBuf>) -> Self {
        Self::Scripted(ScriptedProfileRunner { log_path })
    }
}

impl AndroidCommandRunner for SelectedProfileRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
        match self {
            Self::Real(runner) => runner.run(invocation),
            #[cfg(debug_assertions)]
            Self::Scripted(runner) => runner.run(invocation),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct JsonlProfileLedger {
    module_root: PathBuf,
    path: PathBuf,
    fail_before_mutation: bool,
}

impl JsonlProfileLedger {
    pub(super) fn new(module_root: &Path) -> Self {
        Self {
            module_root: module_root.to_path_buf(),
            path: module_root.join(PROFILE_LEDGER_RELATIVE_PATH),
            fail_before_mutation: false,
        }
    }

    pub(super) fn failing_for_test(module_root: &Path) -> Self {
        Self {
            module_root: module_root.to_path_buf(),
            path: module_root.join(PROFILE_LEDGER_RELATIVE_PATH),
            fail_before_mutation: true,
        }
    }

    pub(super) fn path(module_root: &Path) -> PathBuf {
        module_root.join(PROFILE_LEDGER_RELATIVE_PATH)
    }

    pub(in crate::profile_execute) fn preflight_for_append(
        module_root: &Path,
    ) -> Result<(), ProfileError> {
        Self::new(module_root).ensure_safe_for_append()
    }

    fn ensure_safe_for_append(&self) -> Result<(), ProfileError> {
        ensure_module_child_path(&self.module_root, &self.path)?;
        let parent = self.path.parent().ok_or_else(|| ProfileError::Runner {
            detail: "profile ledger sink failed: path has no parent".to_owned(),
        })?;
        safe_create_dir_all(parent)?;
        ensure_profile_ledger_leaf_is_not_symlink(&self.path)
    }
}

impl ProfileLedgerSink for JsonlProfileLedger {
    fn append(&self, record: String) -> Result<(), ProfileError> {
        if self.fail_before_mutation {
            return Err(ProfileError::Runner {
                detail: "profile ledger sink failed".to_owned(),
            });
        }
        self.ensure_safe_for_append()?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .custom_flags(O_NOFOLLOW)
            .open(&self.path)
            .map_err(|source| profile_ledger_error(&source))?;
        writeln!(file, "{record}").map_err(|source| profile_ledger_error(&source))?;
        Ok(())
    }
}

fn ensure_profile_ledger_leaf_is_not_symlink(path: &Path) -> Result<(), ProfileError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(ProfileError::Runner {
            detail: format!(
                "profile ledger sink failed: profile ledger path is a symlink: {}",
                path.display()
            ),
        }),
        Ok(_metadata) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(profile_ledger_error(&source)),
    }
}

fn profile_ledger_error(source: &std::io::Error) -> ProfileError {
    ProfileError::Runner {
        detail: format!("profile ledger sink failed: {source}"),
    }
}

fn ensure_module_child_path(module_root: &Path, path: &Path) -> Result<(), ProfileError> {
    if path.starts_with(module_root) && !has_parent_component(path) {
        return Ok(());
    }
    Err(ProfileError::Runner {
        detail: "profile ledger sink failed: path escaped module root".to_owned(),
    })
}

fn safe_create_dir_all(path: &Path) -> Result<(), ProfileError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if current.as_os_str().is_empty() {
            continue;
        }
        ensure_directory_component(current.as_path())?;
    }
    Ok(())
}

fn ensure_directory_component(path: &Path) -> Result<(), ProfileError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(ProfileError::Runner {
            detail: format!(
                "profile ledger sink failed: path component is a symlink: {}",
                path.display()
            ),
        }),
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_metadata) => Err(ProfileError::Runner {
            detail: format!(
                "profile ledger sink failed: path component is not a directory: {}",
                path.display()
            ),
        }),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(path).map_err(|source| profile_ledger_error(&source))
        }
        Err(source) => Err(profile_ledger_error(&source)),
    }
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone)]
pub(in crate::profile_execute) struct ScriptedProfileRunner {
    log_path: Option<PathBuf>,
}

#[cfg(debug_assertions)]
impl AndroidCommandRunner for ScriptedProfileRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
        append_test_runner_call(self.log_path.as_deref(), invocation)?;
        let argv = invocation.argv();
        let stdout = scripted_stdout(argv.as_slice());
        Ok(CommandOutput::success(stdout, ""))
    }
}

#[cfg(debug_assertions)]
fn scripted_stdout(argv: &[String]) -> String {
    if argv.get(1).is_some_and(|arg| arg == "path")
        && let Some(package) = argv.get(2)
    {
        return format!("package:/data/app/{package}/base.apk\n");
    }
    String::new()
}

#[cfg(debug_assertions)]
fn append_test_runner_call(
    path: Option<&Path>,
    invocation: &CommandInvocation,
) -> Result<(), CommandRunnerError> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| test_runner_error(&source))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| test_runner_error(&source))?;
    writeln!(file, "{}", invocation.argv().join(" ")).map_err(|source| test_runner_error(&source))
}

#[cfg(debug_assertions)]
fn test_runner_error(source: &std::io::Error) -> CommandRunnerError {
    CommandRunnerError::Unavailable {
        detail: source.to_string(),
    }
}
