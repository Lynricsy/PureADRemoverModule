use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::DaemonError;
use crate::file_rule_integration::apply::{FileRuleApplyExecutor, FileRuleApplyOutcome};
use crate::file_rule_integration::loader::{collect_file_rules, count_skipped_rules};
use crate::file_rule_integration::planner::{ApplyFileAction, DryRunFileAction, FileRulePlanner};

/// 文件规则 daemon 运行模式。
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum FileRuleDaemonMode {
    /// 只输出计划，不执行真实文件动作。
    DryRun,
    /// 执行真实文件动作并写恢复账本。
    Apply {
        /// 恢复账本路径。
        ledger_path: PathBuf,
    },
}

/// 文件规则 daemon 配置。
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileRuleDaemonConfig {
    android_root: PathBuf,
    rule_roots: Vec<PathBuf>,
    mode: FileRuleDaemonMode,
    debounce: Duration,
}

impl FileRuleDaemonConfig {
    /// 创建文件规则 daemon 配置。
    pub fn new(
        android_root: PathBuf,
        rule_roots: Vec<PathBuf>,
        mode: FileRuleDaemonMode,
        debounce: Duration,
    ) -> Result<Self, DaemonError> {
        if rule_roots.is_empty() {
            return Err(DaemonError::EmptyRuleRoots);
        }
        if debounce.is_zero() {
            return Err(DaemonError::EmptyDebounce);
        }
        Ok(Self {
            android_root,
            rule_roots,
            mode,
            debounce,
        })
    }

    /// 准备文件规则 daemon 运行态。
    pub fn prepare(&self) -> Result<FileRuleDaemonRuntime, DaemonError> {
        ensure_dir(&self.android_root, DaemonPathKind::AndroidRoot)?;
        let rules = collect_file_rules(&self.rule_roots)?;
        let skipped_high_frequency_rule_count = count_skipped_rules(&self.rule_roots)?;
        let planner = FileRulePlanner::new(self.android_root.clone(), rules)?;
        let watch_roots = planner.watch_roots();
        let apply_executor = match &self.mode {
            FileRuleDaemonMode::DryRun => None,
            FileRuleDaemonMode::Apply { ledger_path } => Some(FileRuleApplyExecutor::new(
                ledger_path.clone(),
                self.android_root.clone(),
            )),
        };
        Ok(FileRuleDaemonRuntime {
            debounce: self.debounce,
            file_rule_count: planner.rule_count(),
            skipped_high_frequency_rule_count,
            planner,
            watch_roots,
            apply_executor,
        })
    }
}

/// 已准备好的文件规则 daemon 运行态。
#[derive(Debug, Clone)]
pub struct FileRuleDaemonRuntime {
    debounce: Duration,
    file_rule_count: usize,
    skipped_high_frequency_rule_count: usize,
    planner: FileRulePlanner,
    watch_roots: Vec<PathBuf>,
    apply_executor: Option<FileRuleApplyExecutor>,
}

impl FileRuleDaemonRuntime {
    pub(crate) fn dry_run_for_paths(
        &self,
        changed_paths: &[PathBuf],
    ) -> Result<Vec<DryRunFileAction>, DaemonError> {
        self.planner.dry_run_for_paths(changed_paths)
    }

    pub(crate) fn apply_for_paths(
        &self,
        changed_paths: &[PathBuf],
    ) -> Result<Vec<FileRuleApplyOutcome>, DaemonError> {
        let Some(executor) = &self.apply_executor else {
            return Ok(Vec::new());
        };
        let actions: Vec<ApplyFileAction> = self.planner.apply_for_paths(changed_paths)?;
        executor.execute(&actions)
    }

    pub(crate) const fn is_apply_mode(&self) -> bool {
        self.apply_executor.is_some()
    }

    pub(crate) const fn debounce(&self) -> Duration {
        self.debounce
    }

    /// 返回高频 watcher root。
    pub fn watch_roots(&self) -> &[PathBuf] {
        &self.watch_roots
    }

    /// 返回进入高频 watcher 的文件类规则数量。
    pub const fn file_rule_count(&self) -> usize {
        self.file_rule_count
    }

    /// 返回被高频 watcher 跳过的非文件类规则数量。
    pub const fn skipped_high_frequency_rule_count(&self) -> usize {
        self.skipped_high_frequency_rule_count
    }
}

fn ensure_dir(path: &Path, kind: DaemonPathKind) -> Result<(), DaemonError> {
    let metadata = fs::metadata(path).map_err(|source| kind.missing(path, source))?;
    if metadata.is_dir() {
        return Ok(());
    }
    Err(kind.not_directory(path))
}

#[derive(Debug, Clone, Copy)]
enum DaemonPathKind {
    AndroidRoot,
}

impl DaemonPathKind {
    fn missing(self, path: &Path, source: std::io::Error) -> DaemonError {
        match self {
            Self::AndroidRoot => DaemonError::AndroidRootMissing {
                path: path.to_path_buf(),
                source,
            },
        }
    }

    fn not_directory(self, path: &Path) -> DaemonError {
        match self {
            Self::AndroidRoot => DaemonError::AndroidRootNotDirectory {
                path: path.to_path_buf(),
            },
        }
    }
}
