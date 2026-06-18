use crate::RuleCategory;
use crate::error::RuleParseError;

pub(super) fn validate_schedule(
    category: RuleCategory,
    schedule: Option<&str>,
) -> Result<(), RuleParseError> {
    let Some(schedule) = schedule else {
        return Ok(());
    };
    if category != RuleCategory::Sqlite {
        return Err(RuleParseError::InvalidTarget {
            category,
            reason: "only sqlite rules may define schedule",
        });
    }
    match schedule {
        "manual" | "boot_once" | "low_frequency" => Ok(()),
        _ => Err(RuleParseError::InvalidTarget {
            category,
            reason: "unsupported sqlite schedule",
        }),
    }
}
