use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::scheduler::config::SchedulerConfig;
use crate::scheduler::decision::ScheduleDecision;
use crate::scheduler::jitter::{JitterSource, SystemJitter};
use crate::scheduler::policy::{MaintenanceJobKind, MaintenanceSchedule, SchedulePolicy};

const MAX_BACKOFF_SHIFT: u32 = 31;

/// 当前失败退避状态。
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum BackoffOutcome {
    /// 当前没有失败重试。
    NotRetrying,
    /// 当前正在等待退避窗口。
    Waiting,
    /// 当前退避已经到达配置上限。
    Capped,
}

/// 低频维护调度器。
#[derive(Debug, Clone)]
pub struct MaintenanceScheduler<J = SystemJitter> {
    config: SchedulerConfig,
    jitter: J,
    states: HashMap<ScheduleKey, ScheduleState>,
}

impl<J: JitterSource> MaintenanceScheduler<J> {
    /// 使用显式配置和 jitter 来源构造调度器。
    #[must_use]
    pub fn new(config: SchedulerConfig, jitter: J) -> Self {
        Self {
            config,
            jitter,
            states: HashMap::new(),
        }
    }

    /// 计算任务下一次是否应该由自动调度器执行。
    pub fn next_due(&mut self, policy: &SchedulePolicy, now: SystemTime) -> ScheduleDecision {
        match policy.schedule() {
            MaintenanceSchedule::Manual => ScheduleDecision::NotScheduled,
            MaintenanceSchedule::BootOnce => self.next_boot_once(*policy, now),
            MaintenanceSchedule::LowFrequency => self.next_low_frequency(*policy, now),
        }
    }

    /// 记录一次成功执行。
    pub fn record_success(&mut self, policy: &SchedulePolicy, now: SystemTime) {
        let next = match policy.schedule() {
            MaintenanceSchedule::LowFrequency => Some(self.regular_delay()),
            MaintenanceSchedule::BootOnce | MaintenanceSchedule::Manual => None,
        };
        let state = self.state_mut(*policy);
        state.last_success = Some(now);
        state.consecutive_failures = 0;
        state.current_backoff = None;
        state.next = next.map(|delay| ScheduledDelay::new(now, delay));
    }

    /// 记录一次失败执行并更新退避计划。
    pub fn record_failure(&mut self, policy: &SchedulePolicy, now: SystemTime) {
        let failures = self.next_failure_count(*policy);
        let next = match policy.schedule() {
            MaintenanceSchedule::Manual => None,
            MaintenanceSchedule::BootOnce | MaintenanceSchedule::LowFrequency => {
                Some(self.failure_delay(failures))
            }
        };
        let state = self.state_mut(*policy);
        state.last_failure = Some(now);
        state.consecutive_failures = failures;
        state.current_backoff = next;
        state.next = next.map(|delay| ScheduledDelay::new(now, delay));
    }

    /// 返回任务当前退避状态。
    #[must_use]
    pub fn backoff_outcome(&self, policy: &SchedulePolicy) -> BackoffOutcome {
        let Some(state) = self.states.get(&ScheduleKey::from_policy(*policy)) else {
            return BackoffOutcome::NotRetrying;
        };
        if state.consecutive_failures == 0 {
            return BackoffOutcome::NotRetrying;
        }
        match state.current_backoff {
            Some(delay) if delay >= self.config.failure_backoff_cap() => BackoffOutcome::Capped,
            Some(_delay) => BackoffOutcome::Waiting,
            None => BackoffOutcome::NotRetrying,
        }
    }

    fn next_boot_once(&self, policy: SchedulePolicy, now: SystemTime) -> ScheduleDecision {
        let Some(state) = self.states.get(&ScheduleKey::from_policy(policy)) else {
            return ScheduleDecision::DueNow;
        };
        if let Some(next) = state.next {
            return next.decision(now);
        }
        if state.last_success.is_some() {
            ScheduleDecision::NotScheduled
        } else {
            ScheduleDecision::DueNow
        }
    }

    fn next_low_frequency(&mut self, policy: SchedulePolicy, now: SystemTime) -> ScheduleDecision {
        let key = ScheduleKey::from_policy(policy);
        if let Some(next) = self.states.get(&key).and_then(|state| state.next) {
            return next.decision(now);
        }
        let has_history = self
            .states
            .get(&key)
            .is_some_and(ScheduleState::has_history);
        if !has_history && self.config.startup_catch_up() {
            return ScheduleDecision::DueNow;
        }
        let delay = self.regular_delay();
        self.states.entry(key).or_default().next = Some(ScheduledDelay::new(now, delay));
        ScheduleDecision::due_after(delay)
    }

    fn regular_delay(&mut self) -> Duration {
        self.with_jitter(self.config.low_frequency_interval())
    }

    fn failure_delay(&mut self, failures: u32) -> Duration {
        let exponent = failures.saturating_sub(1).min(MAX_BACKOFF_SHIFT);
        let multiplier = 1_u32
            .checked_shl(exponent)
            .map_or(u32::MAX, std::convert::identity);
        let base = self
            .config
            .low_frequency_interval()
            .saturating_mul(multiplier)
            .min(self.config.failure_backoff_cap());
        self.with_jitter(base)
            .min(self.config.failure_backoff_cap())
    }

    fn with_jitter(&mut self, base: Duration) -> Duration {
        base.saturating_add(self.jitter.jitter(self.config.jitter_upper_bound()))
    }

    fn next_failure_count(&self, policy: SchedulePolicy) -> u32 {
        self.states
            .get(&ScheduleKey::from_policy(policy))
            .map_or(1, |state| state.consecutive_failures.saturating_add(1))
    }

    fn state_mut(&mut self, policy: SchedulePolicy) -> &mut ScheduleState {
        self.states
            .entry(ScheduleKey::from_policy(policy))
            .or_default()
    }
}

impl Default for MaintenanceScheduler<SystemJitter> {
    fn default() -> Self {
        Self::new(SchedulerConfig::default(), SystemJitter::default())
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
struct ScheduleKey {
    kind: MaintenanceJobKind,
    schedule: MaintenanceSchedule,
}

impl ScheduleKey {
    const fn from_policy(policy: SchedulePolicy) -> Self {
        Self {
            kind: policy.kind(),
            schedule: policy.schedule(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
struct ScheduleState {
    last_success: Option<SystemTime>,
    last_failure: Option<SystemTime>,
    consecutive_failures: u32,
    current_backoff: Option<Duration>,
    next: Option<ScheduledDelay>,
}

impl ScheduleState {
    const fn has_history(&self) -> bool {
        self.last_success.is_some() || self.last_failure.is_some()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ScheduledDelay {
    anchor: SystemTime,
    delay: Duration,
}

impl ScheduledDelay {
    const fn new(anchor: SystemTime, delay: Duration) -> Self {
        Self { anchor, delay }
    }

    fn decision(self, now: SystemTime) -> ScheduleDecision {
        let elapsed = now
            .duration_since(self.anchor)
            .map_or(Duration::ZERO, std::convert::identity);
        ScheduleDecision::due_after(self.delay.saturating_sub(elapsed))
    }
}
