use crate::error::RuleParseError;
use crate::raw::RawSource;
use crate::validation::{require_text, validate_optional_text};

/// 规则来源元数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSource {
    source: String,
    source_file: Option<String>,
    zip_entry: Option<String>,
    line_or_pattern: String,
}

impl RuleSource {
    pub(crate) fn from_raw(raw: RawSource) -> Result<Self, RuleParseError> {
        require_text("source.source", &raw.source)?;
        require_text("source.source_line_or_pattern", &raw.source_line_or_pattern)?;
        validate_optional_text("source.source_file", raw.source_file.as_deref())?;
        validate_optional_text("source.zip_entry", raw.zip_entry.as_deref())?;
        if raw.source_file.is_none() && raw.zip_entry.is_none() {
            return Err(RuleParseError::MissingSourceLocation);
        }
        Ok(Self {
            source: raw.source,
            source_file: raw.source_file,
            zip_entry: raw.zip_entry,
            line_or_pattern: raw.source_line_or_pattern,
        })
    }

    /// 返回来源快照名。
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 返回普通文件来源路径。
    pub fn source_file(&self) -> Option<&str> {
        self.source_file.as_deref()
    }

    /// 返回 zip 内部条目路径。
    pub fn zip_entry(&self) -> Option<&str> {
        self.zip_entry.as_deref()
    }

    /// 返回来源行号、可搜索片段或片段哈希。
    pub fn line_or_pattern(&self) -> &str {
        &self.line_or_pattern
    }
}
