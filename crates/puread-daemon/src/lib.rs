#![doc = "`PureAD` daemon crate。"]

mod config;
mod error;
mod event;
mod event_loop;
mod file_rule_integration;
mod scheduler;
mod signals;

pub use config::EventLoopConfig;
pub use error::DaemonError;
pub use event::DaemonEvent;
pub use event_loop::{EventLoop, EventLoopHandle};
pub use file_rule_integration::{
    ApplyFileAction, DryRunFileAction, FileRuleApplyOutcome, FileRuleDaemonConfig,
    FileRuleDaemonMode, FileRuleDaemonRuntime,
};
pub use scheduler::{
    BackoffOutcome, JitterSource, MaintenanceJobKind, MaintenanceSchedule, MaintenanceScheduler,
    ScheduleDecision, SchedulePolicy, SchedulerConfig, SystemJitter,
};
pub use signals::SignalForwarder;

/// Daemon crate 标识。
#[must_use]
pub const fn crate_name() -> &'static str {
    "puread-daemon"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn crate_name_returns_daemon_crate_identifier_when_smoke_test_runs() {
        // Given: the daemon crate is compiled as a workspace member.
        // When: its smoke-test API is called.
        let name = crate_name();

        // Then: the observable identifier is stable.
        assert_eq!(name, "puread-daemon");
    }
}
