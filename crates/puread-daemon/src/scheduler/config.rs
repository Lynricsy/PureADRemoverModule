use std::time::Duration;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const MINIMUM_LOW_FREQUENCY_INTERVAL: Duration = Duration::from_secs(6 * SECONDS_PER_HOUR);
const DEFAULT_FAILURE_BACKOFF_CAP: Duration = Duration::from_secs(24 * SECONDS_PER_HOUR);
const DEFAULT_JITTER_UPPER_BOUND: Duration = Duration::from_secs(30 * SECONDS_PER_MINUTE);

/// 低频维护调度配置。
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SchedulerConfig {
    low_frequency_interval: Duration,
    failure_backoff_cap: Duration,
    jitter_upper_bound: Duration,
    startup_catch_up: bool,
}

impl SchedulerConfig {
    /// 构造配置，低频间隔会被钳制到六小时下限。
    #[must_use]
    pub fn new(
        low_frequency_interval: Duration,
        failure_backoff_cap: Duration,
        jitter_upper_bound: Duration,
        startup_catch_up: bool,
    ) -> Self {
        let low_frequency_interval = max_duration(
            low_frequency_interval,
            Self::minimum_low_frequency_interval(),
        );
        let failure_backoff_cap = clamp_duration(
            failure_backoff_cap,
            Self::minimum_low_frequency_interval(),
            DEFAULT_FAILURE_BACKOFF_CAP,
        );
        Self {
            low_frequency_interval,
            failure_backoff_cap,
            jitter_upper_bound,
            startup_catch_up,
        }
    }

    /// 返回允许的最低低频维护间隔。
    #[must_use]
    pub const fn minimum_low_frequency_interval() -> Duration {
        MINIMUM_LOW_FREQUENCY_INTERVAL
    }

    pub(super) const fn low_frequency_interval(self) -> Duration {
        self.low_frequency_interval
    }

    pub(super) const fn failure_backoff_cap(self) -> Duration {
        self.failure_backoff_cap
    }

    pub(super) const fn jitter_upper_bound(self) -> Duration {
        self.jitter_upper_bound
    }

    pub(super) const fn startup_catch_up(self) -> bool {
        self.startup_catch_up
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self::new(
            MINIMUM_LOW_FREQUENCY_INTERVAL,
            DEFAULT_FAILURE_BACKOFF_CAP,
            DEFAULT_JITTER_UPPER_BOUND,
            true,
        )
    }
}

fn max_duration(left: Duration, right: Duration) -> Duration {
    if left >= right { left } else { right }
}

fn clamp_duration(value: Duration, lower: Duration, upper: Duration) -> Duration {
    max_duration(value.min(upper), lower)
}
