use crate::file_actions::metadata::MetadataOperation;

/// 文件动作执行模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// 只规划，不修改文件系统或账本。
    DryRun,
    /// 写账本并执行真实修改。
    Apply,
}

/// 文件动作状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileActionStatus {
    /// 已规划。
    Planned,
    /// 已真实执行。
    Applied,
    /// 目标不存在或无需处理。
    Skipped,
}

/// 文件动作执行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileActionOutcome {
    mode: ExecutionMode,
    status: FileActionStatus,
    will_mutate: bool,
    metadata_operations: Vec<MetadataOperation>,
}

impl FileActionOutcome {
    pub(super) const fn planned() -> Self {
        Self {
            mode: ExecutionMode::DryRun,
            status: FileActionStatus::Planned,
            will_mutate: false,
            metadata_operations: Vec::new(),
        }
    }

    pub(super) const fn applied(metadata_operations: Vec<MetadataOperation>) -> Self {
        Self {
            mode: ExecutionMode::Apply,
            status: FileActionStatus::Applied,
            will_mutate: true,
            metadata_operations,
        }
    }

    pub(super) const fn skipped() -> Self {
        Self {
            mode: ExecutionMode::Apply,
            status: FileActionStatus::Skipped,
            will_mutate: false,
            metadata_operations: Vec::new(),
        }
    }

    /// 返回执行模式。
    #[must_use]
    pub const fn mode(&self) -> ExecutionMode {
        self.mode
    }

    /// 返回执行状态。
    #[must_use]
    pub const fn status(&self) -> FileActionStatus {
        self.status
    }

    /// 返回该结果是否修改了目标文件系统。
    #[must_use]
    pub const fn will_mutate(&self) -> bool {
        self.will_mutate
    }

    /// 返回元数据封装操作结果。
    #[must_use]
    pub fn metadata_operations(&self) -> &[MetadataOperation] {
        &self.metadata_operations
    }
}
