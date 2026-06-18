use std::path::Path;

use crate::command_runner::{
    AndroidCommandAdapter, AndroidCommandRunner, CommandOutput, GetpropAdapter, SettingsAdapter,
};
use crate::profiles::error::ProfileError;
use crate::profiles::executor::shared::parse_record;
use crate::profiles::executor::{AndroidProfileExecutor, ProfileLedgerSink};
use crate::profiles::record::{
    ProfileOperation, ProfileRecord, RomSettingRecord, RomSkippedRecord, SharedPrefsBoolRecord,
};
use crate::profiles::rules::{RomProfileAction, RomProfileRule, RomSettingsRule};
use crate::profiles::xml::{commit_bool, plan_bool, preflight_bool_commit, restore_from_backup};

impl<R, L> AndroidProfileExecutor<'_, R, L>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    /// 应用 ROM profile。
    pub fn apply_rom(&self, rule: &RomProfileRule) -> Result<ProfileOperation, ProfileError> {
        if !self.rom_matches(rule)? {
            return self.skipped(&ProfileRecord::RomSkipped(RomSkippedRecord {
                rule_id: rule.id.clone(),
                matcher: rule.matcher.name.clone(),
                reason: "rom_property_empty".to_owned(),
            }));
        }
        match &rule.action {
            RomProfileAction::Settings(settings) => self.apply_rom_settings(rule, settings),
            RomProfileAction::SharedPrefsBool(xml) => {
                let plan = plan_bool(&xml.path, &xml.key, xml.value, &xml.backup_dir, &rule.id)?;
                preflight_bool_commit(&xml.path, &xml.backup_dir, &plan)?;
                let record = ProfileRecord::SharedPrefsBool(SharedPrefsBoolRecord {
                    rule_id: rule.id.clone(),
                    matcher: rule.matcher.name.clone(),
                    path: xml.path.display().to_string(),
                    key: xml.key.clone(),
                    applied_value: xml.value,
                    original_value: plan.original_value,
                    original_sha256: plan.original_sha256.clone(),
                    backup_path: plan.backup_path.display().to_string(),
                });
                let operation = self.applied(&record)?;
                commit_bool(&xml.path, &xml.backup_dir, &plan)?;
                Ok(operation)
            }
        }
    }

    /// 恢复 ROM profile。
    pub fn restore_rom(&self, record: &str) -> Result<(), ProfileError> {
        match parse_record(record)? {
            ProfileRecord::RomSetting(record) => {
                SettingsAdapter::new(
                    record.namespace,
                    &record.key,
                    &record.applied_value,
                    record.original_value.as_deref(),
                )?
                .restore(self.runner)?;
            }
            ProfileRecord::SharedPrefsBool(record) => {
                restore_from_backup(Path::new(&record.path), Path::new(&record.backup_path))?;
            }
            ProfileRecord::RomSkipped(_)
            | ProfileRecord::AppOp(_)
            | ProfileRecord::Component(_) => {}
        }
        Ok(())
    }

    fn apply_rom_settings(
        &self,
        rule: &RomProfileRule,
        settings: &RomSettingsRule,
    ) -> Result<ProfileOperation, ProfileError> {
        let probe = SettingsAdapter::new(settings.namespace, &settings.key, &settings.value, None)?
            .probe(self.runner)?;
        let original_value = normalize_settings_value(probe.output());
        let record = ProfileRecord::RomSetting(RomSettingRecord {
            rule_id: rule.id.clone(),
            matcher: rule.matcher.name.clone(),
            namespace: settings.namespace,
            key: settings.key.clone(),
            applied_value: settings.value.clone(),
            original_value,
        });
        let operation = self.applied(&record)?;
        SettingsAdapter::new(
            settings.namespace,
            &settings.key,
            &settings.value,
            match &record {
                ProfileRecord::RomSetting(record) => record.original_value.as_deref(),
                _ => None,
            },
        )?
        .apply(self.runner)?;
        Ok(operation)
    }

    fn rom_matches(&self, rule: &RomProfileRule) -> Result<bool, ProfileError> {
        let outcome = GetpropAdapter::new(&rule.matcher.property)?.probe(self.runner)?;
        Ok(outcome
            .output()
            .map(CommandOutput::stdout)
            .is_some_and(|stdout| !stdout.trim().is_empty()))
    }
}

fn normalize_settings_value(output: Option<&CommandOutput>) -> Option<String> {
    let value = output.map(CommandOutput::stdout)?.trim();
    if value.is_empty() || value == "null" {
        return None;
    }
    Some(value.to_owned())
}
