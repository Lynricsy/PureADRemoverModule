use std::time::Duration;

/// 调度器对单个维护任务给出的下一步决策。
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum ScheduleDecision {
    /// 当前就应该执行。
    DueNow,
    /// 等待指定时长后执行。
    DueAfter(Duration),
    /// 不应由自动调度器执行。
    NotScheduled,
}

impl ScheduleDecision {
    pub(super) const fn due_after(delay: Duration) -> Self {
        if delay.is_zero() {
            Self::DueNow
        } else {
            Self::DueAfter(delay)
        }
    }
}
