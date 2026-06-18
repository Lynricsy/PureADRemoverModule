#![doc = "`puread-daemon` 低频维护调度测试。"]

use std::time::{Duration, SystemTime};

use puread_daemon::{
    BackoffOutcome, JitterSource, MaintenanceJobKind, MaintenanceScheduler, ScheduleDecision,
    SchedulePolicy, SchedulerConfig,
};

const NOW: SystemTime = SystemTime::UNIX_EPOCH;
const SIX_HOURS: Duration = Duration::from_secs(6 * 60 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn scheduler_boot_once_runs_only_before_first_success_when_startup_is_evaluated() {
    // Given: a boot-once SQLite maintenance job has never succeeded.
    let mut scheduler = MaintenanceScheduler::new(
        SchedulerConfig::default(),
        FixedJitter::new(Duration::from_secs(15 * 60)),
    );
    let job = SchedulePolicy::boot_once(MaintenanceJobKind::Sqlite);

    // When: startup scheduling is evaluated twice around a success.
    let first = scheduler.next_due(&job, NOW);
    scheduler.record_success(&job, NOW);
    let second = scheduler.next_due(&job, NOW);

    // Then: only the first startup pass is due.
    assert_eq!(first, ScheduleDecision::DueNow);
    assert_eq!(second, ScheduleDecision::NotScheduled);
}

#[test]
fn scheduler_manual_job_never_creates_automatic_plan_when_evaluated() {
    // Given: a manual SQLite maintenance job is configured.
    let mut scheduler = MaintenanceScheduler::new(SchedulerConfig::default(), FixedJitter::zero());
    let job = SchedulePolicy::manual(MaintenanceJobKind::Sqlite);

    // When: automatic scheduling is evaluated before and after success/failure records.
    let first = scheduler.next_due(&job, NOW);
    scheduler.record_failure(&job, NOW);
    scheduler.record_success(&job, NOW);
    let second = scheduler.next_due(&job, NOW);

    // Then: manual work is never automatically planned.
    assert_eq!(first, ScheduleDecision::NotScheduled);
    assert_eq!(second, ScheduleDecision::NotScheduled);
}

#[test]
fn scheduler_low_frequency_rescan_schedules_startup_once_then_waits_at_least_six_hours()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: default low-frequency rescan has startup catch-up enabled.
    let mut scheduler = MaintenanceScheduler::new(
        SchedulerConfig::default(),
        FixedJitter::new(Duration::from_secs(17 * 60)),
    );
    let job = SchedulePolicy::low_frequency(MaintenanceJobKind::Rescan);

    // When: startup is evaluated, then a success is recorded.
    let startup = scheduler.next_due(&job, NOW);
    scheduler.record_success(&job, NOW);
    let scheduled = scheduler.next_due(&job, NOW);

    // Then: first boot catches up once and the regular interval is six hours plus jitter.
    assert_eq!(startup, ScheduleDecision::DueNow);
    assert_due_after_at_least(scheduled, SIX_HOURS)?;
    assert_eq!(
        scheduled,
        ScheduleDecision::DueAfter(SIX_HOURS + Duration::from_secs(17 * 60))
    );
    Ok(())
}

#[test]
fn scheduler_low_frequency_failure_uses_exponential_backoff_capped_at_one_day_with_jitter() {
    // Given: a low-frequency SQLite job repeatedly fails.
    let mut scheduler = MaintenanceScheduler::new(
        SchedulerConfig::default(),
        FixedJitter::new(Duration::from_secs(11 * 60)),
    );
    let job = SchedulePolicy::low_frequency(MaintenanceJobKind::Sqlite);

    // When: failures are recorded from a clean state.
    scheduler.record_failure(&job, NOW);
    let first = scheduler.next_due(&job, NOW);
    scheduler.record_failure(&job, NOW);
    let second = scheduler.next_due(&job, NOW);
    for _attempt in 0..8 {
        scheduler.record_failure(&job, NOW);
    }
    let capped = scheduler.next_due(&job, NOW);

    // Then: backoff doubles, never drops below six hours, caps at one day, and keeps jitter.
    assert_eq!(scheduler.backoff_outcome(&job), BackoffOutcome::Capped);
    assert_eq!(
        first,
        ScheduleDecision::DueAfter(SIX_HOURS + Duration::from_secs(11 * 60))
    );
    assert_eq!(
        second,
        ScheduleDecision::DueAfter(Duration::from_secs(12 * 60 * 60 + 11 * 60))
    );
    assert_eq!(capped, ScheduleDecision::DueAfter(DAY));
}

#[test]
fn scheduler_custom_low_frequency_interval_cannot_go_below_six_hours() {
    // Given: a caller tries to configure a short low-frequency interval.
    let config = SchedulerConfig::new(
        Duration::from_secs(60),
        Duration::from_secs(24 * 60 * 60),
        Duration::from_secs(10 * 60),
        true,
    );
    let mut scheduler = MaintenanceScheduler::new(config, FixedJitter::zero());
    let job = SchedulePolicy::low_frequency(MaintenanceJobKind::Rescan);

    // When: the job succeeds and asks for its next automatic run.
    scheduler.record_success(&job, NOW);
    let scheduled = scheduler.next_due(&job, NOW);

    // Then: the generated plan is clamped to the six-hour floor.
    assert_eq!(scheduled, ScheduleDecision::DueAfter(SIX_HOURS));
}

#[test]
fn scheduler_failure_backoff_never_exceeds_one_day_when_custom_interval_is_longer() {
    // Given: a custom interval is longer than the failure backoff ceiling.
    let config = SchedulerConfig::new(
        Duration::from_secs(36 * 60 * 60),
        Duration::from_secs(72 * 60 * 60),
        Duration::from_secs(15 * 60),
        true,
    );
    let mut scheduler =
        MaintenanceScheduler::new(config, FixedJitter::new(Duration::from_secs(15)));
    let job = SchedulePolicy::low_frequency(MaintenanceJobKind::Sqlite);

    // When: the job fails and requests its retry schedule.
    scheduler.record_failure(&job, NOW);
    let scheduled = scheduler.next_due(&job, NOW);

    // Then: the failure retry is capped to one day instead of using the long interval.
    assert_eq!(scheduled, ScheduleDecision::DueAfter(DAY));
    assert_eq!(scheduler.backoff_outcome(&job), BackoffOutcome::Capped);
}

fn assert_due_after_at_least(
    decision: ScheduleDecision,
    minimum: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let ScheduleDecision::DueAfter(delay) = decision else {
        return Err(format!("expected scheduled delay, got {decision:?}").into());
    };
    assert!(
        delay >= minimum,
        "scheduled delay {delay:?} is below minimum {minimum:?}"
    );
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct FixedJitter {
    value: Duration,
}

impl FixedJitter {
    const fn new(value: Duration) -> Self {
        Self { value }
    }

    const fn zero() -> Self {
        Self {
            value: Duration::ZERO,
        }
    }
}

impl JitterSource for FixedJitter {
    fn jitter(&mut self, _upper_bound: Duration) -> Duration {
        self.value
    }
}
