/// 低频维护任务类型。
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MaintenanceJobKind {
    /// `SQLite` 广告数据库维护。
    Sqlite,
    /// 本地规则补扫。
    Rescan,
}

/// 维护任务调度策略。
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MaintenanceSchedule {
    /// 仅启动后执行一次，成功后不再自动计划。
    BootOnce,
    /// 只能由用户显式触发。
    Manual,
    /// 低频自动维护。
    LowFrequency,
}

/// 单个维护任务的调度声明。
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct SchedulePolicy {
    kind: MaintenanceJobKind,
    schedule: MaintenanceSchedule,
}

impl SchedulePolicy {
    /// 创建启动一次任务。
    #[must_use]
    pub const fn boot_once(kind: MaintenanceJobKind) -> Self {
        Self {
            kind,
            schedule: MaintenanceSchedule::BootOnce,
        }
    }

    /// 创建手动任务。
    #[must_use]
    pub const fn manual(kind: MaintenanceJobKind) -> Self {
        Self {
            kind,
            schedule: MaintenanceSchedule::Manual,
        }
    }

    /// 创建低频任务。
    #[must_use]
    pub const fn low_frequency(kind: MaintenanceJobKind) -> Self {
        Self {
            kind,
            schedule: MaintenanceSchedule::LowFrequency,
        }
    }

    pub(super) const fn kind(self) -> MaintenanceJobKind {
        self.kind
    }

    pub(super) const fn schedule(self) -> MaintenanceSchedule {
        self.schedule
    }
}
