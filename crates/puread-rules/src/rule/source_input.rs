use crate::error::RuleParseError;
use crate::raw::{RawRule, RawSource, RawSourceInput};

pub(super) fn source_from_raw(
    source: RawSourceInput,
    raw: &RawRule,
) -> Result<RawSource, RuleParseError> {
    match source {
        RawSourceInput::Nested(nested) => {
            if raw.source_file.is_some()
                || raw.zip_entry.is_some()
                || raw.source_line_or_pattern.is_some()
            {
                return Err(RuleParseError::InvalidSourceMetadata {
                    reason: "nested source must not be mixed with flat source fields",
                });
            }
            Ok(nested)
        }
        RawSourceInput::Name(source) => {
            let Some(source_line_or_pattern) = raw.source_line_or_pattern.clone() else {
                return Err(RuleParseError::InvalidSourceMetadata {
                    reason: "flat source requires source_line_or_pattern",
                });
            };
            Ok(RawSource {
                source,
                source_file: raw.source_file.clone(),
                zip_entry: raw.zip_entry.clone(),
                source_line_or_pattern,
            })
        }
    }
}
