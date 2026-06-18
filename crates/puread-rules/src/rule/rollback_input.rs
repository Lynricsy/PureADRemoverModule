use crate::RuleCategory;
use crate::error::RuleParseError;
use crate::rollback::RollbackStrategy;

pub(super) fn rollback_strategy_from_raw(
    category: RuleCategory,
    raw: &str,
) -> Result<RollbackStrategy, RuleParseError> {
    match RollbackStrategy::parse(raw) {
        Ok(strategy) => Ok(strategy),
        Err(error) if category == RuleCategory::Sqlite => sqlite_rollback_strategy(raw, error),
        Err(error) => Err(error),
    }
}

fn sqlite_rollback_strategy(
    raw: &str,
    error: RuleParseError,
) -> Result<RollbackStrategy, RuleParseError> {
    if raw.contains("snapshot the original database path") && raw.contains("restore") {
        return Ok(RollbackStrategy::RestoreOriginal);
    }
    Err(error)
}
