use std::path::PathBuf;

use crate::{DryRunFileAction, FileRuleApplyOutcome};

/// daemon 事件循环向上层输出的骨架事件。
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum DaemonEvent {
    /// 事件循环已经完成 watcher 注册并进入阻塞等待。
    Started,

    /// 收到 reload signal。
    ReloadRequested,

    /// 收到 shutdown signal。
    ShutdownRequested,

    /// watcher 收到文件系统变化，路径已经按去抖窗口合并。
    FilesChanged {
        /// 变化事件中涉及的路径。
        paths: Vec<PathBuf>,
    },

    /// dry-run daemon 生成的文件规则计划。
    DryRunFilePlan {
        /// 计划但未执行的文件动作。
        actions: Vec<DryRunFileAction>,
    },

    /// apply daemon 生成的文件规则执行结果。
    FileRuleApplyReport {
        /// 已执行或跳过的文件动作结果。
        outcomes: Vec<FileRuleApplyOutcome>,
    },
}
