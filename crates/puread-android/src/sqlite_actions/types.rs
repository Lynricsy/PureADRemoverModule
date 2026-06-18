use std::path::PathBuf;

use crate::sqlite_actions::error::SqliteActionFailure;
use crate::sqlite_actions::metadata::SqliteTargetMetadata;
use crate::sqlite_actions::target::SqliteActionTarget;

/// `SQLite` 规则动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SqliteAction {
    /// 删除广告数据库。
    Delete,
    /// 写入最小可识别 `SQLite` 数据库。
    MinimalSqlite,
    /// 写入只读占位数据库阻止应用继续写入广告库。
    DenyWrite,
}

/// `SQLite` 动作调度来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SqliteActionSchedule {
    /// 用户手动触发。
    Manual,
    /// 开机后执行一次。
    BootOnce,
    /// 低频任务触发。
    LowFrequency,
    /// 高频文件写入触发，`SQLite` 执行器会拒绝。
    HighFrequency,
}

impl SqliteActionSchedule {
    pub(super) const fn is_allowed(self) -> bool {
        matches!(self, Self::Manual | Self::BootOnce | Self::LowFrequency)
    }

    pub(super) const fn ledger_profile(self) -> &'static str {
        match self {
            Self::Manual => "sqlite:manual",
            Self::BootOnce => "sqlite:boot_once",
            Self::LowFrequency => "sqlite:low_frequency",
            Self::HighFrequency => "sqlite:high_frequency",
        }
    }
}

/// 单条 `SQLite` 动作请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteActionRequest {
    rule_id: String,
    target: SqliteActionTarget,
    action: SqliteAction,
    schedule: SqliteActionSchedule,
}

impl SqliteActionRequest {
    /// 构造动作请求。
    #[must_use]
    pub fn new(
        rule_id: impl Into<String>,
        target: SqliteActionTarget,
        action: SqliteAction,
        schedule: SqliteActionSchedule,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            target,
            action,
            schedule,
        }
    }

    /// 返回规则 ID。
    #[must_use]
    pub const fn rule_id(&self) -> &str {
        self.rule_id.as_str()
    }

    /// 返回目标。
    #[must_use]
    pub const fn target(&self) -> &SqliteActionTarget {
        &self.target
    }

    /// 返回动作。
    #[must_use]
    pub const fn action(&self) -> SqliteAction {
        self.action
    }

    /// 返回调度。
    #[must_use]
    pub const fn schedule(&self) -> SqliteActionSchedule {
        self.schedule
    }
}

/// 单条 `SQLite` 动作执行状态。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqliteActionStatus {
    /// 已执行 mutation。
    Applied,
    /// 没有必要执行 mutation。
    Skipped(String),
    /// 执行失败，批处理会继续后续请求。
    Failed(SqliteActionFailure),
}

/// 单条 `SQLite` 动作输出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteActionOutcome {
    /// 规则 ID。
    pub rule_id: String,
    /// 目标路径。
    pub target: PathBuf,
    /// 动作。
    pub action: SqliteAction,
    /// 调度。
    pub schedule: SqliteActionSchedule,
    /// 执行前元信息。
    pub metadata: Option<SqliteTargetMetadata>,
    /// 执行状态。
    pub status: SqliteActionStatus,
}

/// `SQLite` 批处理报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchReport {
    /// 每条请求的可观察结果。
    pub outcomes: Vec<SqliteActionOutcome>,
    /// 成功执行数量。
    pub succeeded: usize,
    /// 失败数量。
    pub failed: usize,
}
