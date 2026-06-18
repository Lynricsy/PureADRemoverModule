use crate::error::RuleParseError;
use crate::raw::RawDocument;

pub(super) fn validate_document_metadata(raw: &RawDocument) -> Result<(), RuleParseError> {
    if let Some(version) = raw.schema_version
        && version != 1
    {
        return Err(RuleParseError::UnsupportedDocumentMetadata {
            field: "schema_version",
            value: version.to_string(),
        });
    }
    if let Some(kind) = raw.kind.as_deref()
        && kind != "sqlite"
    {
        return Err(RuleParseError::UnsupportedDocumentMetadata {
            field: "kind",
            value: kind.to_owned(),
        });
    }
    Ok(())
}
