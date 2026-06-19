mod skip;

use crate::command_runner::{
    AndroidCommandAdapter, AndroidCommandRunner, AppOpsAdapter, CommandOutput, PmComponentAdapter,
    PmPackageAdapter,
};
use crate::profiles::error::ProfileError;
use crate::profiles::executor::shared::{parse_record, run_required};
use crate::profiles::executor::{AndroidProfileExecutor, ProfileLedgerSink};
use crate::profiles::record::{
    AppOpRecord, AppSkippedRecord, ComponentRecord, HideApplyStatus, PackageEnabledState,
    PackageHiddenState, ProfileOperation, ProfileRecord,
};
use crate::profiles::rules::{AppOpProfileRule, ComponentProfileRule, PmHidePolicy};

use self::skip::package_probe_missing;

impl<R, L> AndroidProfileExecutor<'_, R, L>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    /// 应用 `AppOps` 规则并记录原 mode。
    pub fn apply_appop(&self, rule: &AppOpProfileRule) -> Result<ProfileOperation, ProfileError> {
        let package = PmPackageAdapter::new(&rule.package)?;
        match run_required(self.runner, &package.path_probe()) {
            Ok(output) if !output.stdout().trim().is_empty() => {}
            Ok(_output) => {
                return self.skipped_app(&rule.id, &rule.package, "package_not_installed");
            }
            Err(error) if package_probe_missing(&error) => {
                return self.skipped_app(&rule.id, &rule.package, "package_not_installed");
            }
            Err(error) => return Err(error),
        }
        let adapter = AppOpsAdapter::new(&rule.package, &rule.op, &rule.mode, "default")?;
        let probe = adapter.probe(self.runner).map_err(ProfileError::Command)?;
        let original_mode = parse_appop_mode(probe.output(), &rule.op);
        let record = ProfileRecord::AppOp(AppOpRecord {
            rule_id: rule.id.clone(),
            package: rule.package.clone(),
            op: rule.op.clone(),
            applied_mode: rule.mode.clone(),
            original_mode: original_mode.clone(),
        });
        let operation = self.applied(&record)?;
        AppOpsAdapter::new(&rule.package, &rule.op, &rule.mode, &original_mode)?
            .apply(self.runner)?;
        Ok(operation)
    }

    /// 恢复 `AppOps` 规则。
    pub fn restore_appop(&self, record: &str) -> Result<(), ProfileError> {
        let ProfileRecord::AppOp(record) = parse_record(record)? else {
            return Ok(());
        };
        AppOpsAdapter::new(
            &record.package,
            &record.op,
            &record.applied_mode,
            &record.original_mode,
        )?
        .restore(self.runner)?;
        Ok(())
    }

    /// 应用组件禁用规则。
    pub fn apply_component(
        &self,
        rule: &ComponentProfileRule,
    ) -> Result<ProfileOperation, ProfileError> {
        let package = PmPackageAdapter::new(&rule.package)?;
        match run_required(self.runner, &package.path_probe()) {
            Ok(output) if !output.stdout().trim().is_empty() => {}
            Ok(_output) => {
                return self.skipped_app(&rule.id, &rule.package, "package_not_installed");
            }
            Err(error) if package_probe_missing(&error) => {
                return self.skipped_app(&rule.id, &rule.package, "package_not_installed");
            }
            Err(error) => return Err(error),
        }
        let disabled_probe = run_required(self.runner, &package.disabled_probe())?;
        let hidden_probe = run_required(self.runner, &package.hidden_probe())?;
        let original_enabled = enabled_state(&disabled_probe);
        let original_hidden = hidden_state(&hidden_probe);
        let record = component_record(
            rule,
            original_enabled,
            original_hidden,
            hide_status_for_policy(rule.hide_policy),
        );
        let operation = self.applied(&record)?;
        let hide_status = self.apply_hide_policy(&package, rule.hide_policy);
        let record = component_record(rule, original_enabled, original_hidden, hide_status);
        let record = match hide_status {
            HideApplyStatus::Applied => {
                let record = profile_record_json(&record)?;
                self.ledger.append(record.clone())?;
                record
            }
            HideApplyStatus::NotRequested | HideApplyStatus::SkippedUnavailable => {
                profile_record_json(&record)?
            }
        };
        PmComponentAdapter::new(rule.user_id, &rule.component)?.apply(self.runner)?;
        Ok(ProfileOperation {
            status: operation.status,
            record,
        })
    }

    fn skipped_app(
        &self,
        rule_id: &str,
        package: &str,
        reason: &str,
    ) -> Result<ProfileOperation, ProfileError> {
        self.skipped(&ProfileRecord::AppSkipped(AppSkippedRecord {
            rule_id: rule_id.to_owned(),
            package: package.to_owned(),
            reason: reason.to_owned(),
        }))
    }

    /// 恢复组件状态。
    pub fn restore_component(&self, record: &str) -> Result<(), ProfileError> {
        let ProfileRecord::Component(record) = parse_record(record)? else {
            return Ok(());
        };
        let package = PmPackageAdapter::new(&record.package)?;
        if record.original_hidden == PackageHiddenState::Visible
            && record.hide_status == HideApplyStatus::Applied
        {
            let _ = run_required(self.runner, &package.unhide())?;
        }
        if record.original_enabled == PackageEnabledState::Enabled {
            PmComponentAdapter::new(record.user_id, &record.component)?.restore(self.runner)?;
        }
        Ok(())
    }

    fn apply_hide_policy(
        &self,
        package: &PmPackageAdapter,
        policy: PmHidePolicy,
    ) -> HideApplyStatus {
        match policy {
            PmHidePolicy::DoNotHide => HideApplyStatus::NotRequested,
            PmHidePolicy::TryHide => run_required(self.runner, &package.try_hide())
                .map_or(HideApplyStatus::SkippedUnavailable, |_output| {
                    HideApplyStatus::Applied
                }),
        }
    }
}

const fn hide_status_for_policy(policy: PmHidePolicy) -> HideApplyStatus {
    match policy {
        PmHidePolicy::DoNotHide => HideApplyStatus::NotRequested,
        PmHidePolicy::TryHide => HideApplyStatus::SkippedUnavailable,
    }
}

fn component_record(
    rule: &ComponentProfileRule,
    original_enabled: PackageEnabledState,
    original_hidden: PackageHiddenState,
    hide_status: HideApplyStatus,
) -> ProfileRecord {
    ProfileRecord::Component(ComponentRecord {
        rule_id: rule.id.clone(),
        user_id: rule.user_id,
        package: rule.package.clone(),
        component: rule.component.clone(),
        original_enabled,
        original_hidden,
        hide_status,
    })
}

fn profile_record_json(record: &ProfileRecord) -> Result<String, ProfileError> {
    serde_json::to_string(record).map_err(ProfileError::json)
}

fn parse_appop_mode(output: Option<&CommandOutput>, op: &str) -> String {
    output
        .map(CommandOutput::stdout)
        .and_then(|stdout| stdout.lines().find(|line| line.contains(op)))
        .and_then(parse_mode_from_line)
        .unwrap_or("default")
        .to_owned()
}

fn parse_mode_from_line(line: &str) -> Option<&str> {
    line.split_once(':')
        .map(|(_prefix, suffix)| suffix.trim())
        .filter(|value| !value.is_empty())
}

fn enabled_state(output: &CommandOutput) -> PackageEnabledState {
    if output.stdout().trim().is_empty() {
        return PackageEnabledState::Enabled;
    }
    PackageEnabledState::Disabled
}

fn hidden_state(output: &CommandOutput) -> PackageHiddenState {
    if output.stdout().trim().is_empty() {
        return PackageHiddenState::Visible;
    }
    PackageHiddenState::Hidden
}
