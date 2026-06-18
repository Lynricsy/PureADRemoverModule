use crate::profile_execute::profile_restore::ledger::ProfileLedgerEntry;

#[derive(Debug, Clone)]
pub(super) struct FinalRestoreEntry {
    ledger_entry: ProfileLedgerEntry,
    restored_indexes: Vec<usize>,
}

impl FinalRestoreEntry {
    pub(super) const fn ledger_entry(&self) -> &ProfileLedgerEntry {
        &self.ledger_entry
    }

    pub(super) const fn restored_indexes(&self) -> &[usize] {
        self.restored_indexes.as_slice()
    }
}

pub(super) fn entries(source: &[ProfileLedgerEntry]) -> Vec<FinalRestoreEntry> {
    let mut targets: Vec<FinalRestoreEntry> = Vec::new();
    for (index, entry) in source
        .iter()
        .enumerate()
        .filter(|(_index, entry)| !entry.restored())
    {
        let Some(position) = component_target_position(&targets, entry) else {
            if let Some(entry) = final_entry(index, source) {
                targets.push(entry);
            }
            continue;
        };
        if let (Some(target), Some(entry)) = (targets.get_mut(position), final_entry(index, source))
        {
            *target = entry;
        }
    }
    targets
}

fn final_entry(index: usize, entries: &[ProfileLedgerEntry]) -> Option<FinalRestoreEntry> {
    let entry = entries.get(index)?;
    Some(FinalRestoreEntry {
        ledger_entry: entry.clone(),
        restored_indexes: restored_indexes_for(index, entries),
    })
}

fn restored_indexes_for(index: usize, entries: &[ProfileLedgerEntry]) -> Vec<usize> {
    let Some(entry) = entries.get(index) else {
        return Vec::new();
    };
    let Some(action_key) = component_action_key(entry) else {
        return vec![index];
    };
    entries
        .iter()
        .enumerate()
        .filter(|(_index, entry)| {
            !entry.restored() && component_action_key(entry) == Some(action_key.clone())
        })
        .map(|(index, _entry)| index)
        .collect()
}

fn component_target_position(
    targets: &[FinalRestoreEntry],
    entry: &ProfileLedgerEntry,
) -> Option<usize> {
    let action_key = component_action_key(entry)?;
    targets
        .iter()
        .position(|target| component_action_key(target.ledger_entry()) == Some(action_key.clone()))
}

fn component_action_key(entry: &ProfileLedgerEntry) -> Option<ComponentActionKey> {
    if entry.kind() != "component" {
        return None;
    }
    let value = serde_json::from_str::<serde_json::Value>(entry.raw()).ok()?;
    Some(ComponentActionKey {
        rule_id: string_field(&value, "rule_id")?.to_owned(),
        user_id: u32_field(&value, "user_id")?,
        package: string_field(&value, "package")?.to_owned(),
        component: string_field(&value, "component")?.to_owned(),
    })
}

fn string_field<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

fn u32_field(value: &serde_json::Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentActionKey {
    rule_id: String,
    user_id: u32,
    package: String,
    component: String,
}
