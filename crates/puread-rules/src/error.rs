use puread_core::error::ModelError;
use thiserror::Error;

use crate::RuleCategory;

/// TOML 规则文档解析和 schema 校验错误。
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RuleParseError {
    /// TOML 语法或 serde schema 错误。
    #[error("invalid TOML rule document: {source}")]
    Toml {
        /// 原始 TOML 解析错误。
        #[source]
        source: Box<toml::de::Error>,
    },
    /// core 类型模型拒绝了输入值。
    #[error("rule model rejected input: {source}")]
    Model {
        /// 原始模型错误。
        #[source]
        source: Box<ModelError>,
    },
    /// 规则类别属于项目禁止能力。
    #[error("forbidden rule category: {category}")]
    ForbiddenCategory {
        /// 被拒绝的类别名。
        category: String,
    },
    /// 规则字段属于项目禁止能力。
    #[error("forbidden capability field: {field}")]
    ForbiddenField {
        /// 被拒绝的字段名。
        field: String,
    },
    /// 规则类别不在允许矩阵中。
    #[error("unsupported rule category: {category}")]
    UnsupportedCategory {
        /// 被拒绝的类别名。
        category: String,
    },
    /// 必填元数据为空。
    #[error("rule metadata field must not be empty: {field}")]
    EmptyMetadata {
        /// 出错字段。
        field: &'static str,
    },
    /// source 元数据缺少普通文件路径或 zip entry。
    #[error("source metadata must include source_file or zip_entry")]
    MissingSourceLocation,
    /// source 元数据字段组合不符合支持的规则 schema。
    #[error("invalid source metadata: {reason}")]
    InvalidSourceMetadata {
        /// 拒绝原因。
        reason: &'static str,
    },
    /// 顶层规则文档元数据不受支持。
    #[error("unsupported document metadata {field}={value}")]
    UnsupportedDocumentMetadata {
        /// 顶层字段名。
        field: &'static str,
        /// 顶层字段值。
        value: String,
    },
    /// 规则动作与类别不兼容。
    #[error("action {action} is not allowed for category {category}")]
    ActionCategoryMismatch {
        /// 规则类别。
        category: RuleCategory,
        /// 动作名。
        action: &'static str,
    },
    /// 规则 profile 与类别不兼容。
    #[error("profile {profile} is not allowed for category {category}")]
    ProfileCategoryMismatch {
        /// 规则类别。
        category: RuleCategory,
        /// profile 名。
        profile: &'static str,
    },
    /// 默认启用状态不符合风险边界。
    #[error("default_enabled={default_enabled} is not allowed for category {category}")]
    DefaultEnabledMismatch {
        /// 规则类别。
        category: RuleCategory,
        /// 原始默认启用状态。
        default_enabled: bool,
    },
    /// 目标字段组合不符合类别 schema。
    #[error("invalid target for category {category}: {reason}")]
    InvalidTarget {
        /// 规则类别。
        category: RuleCategory,
        /// 拒绝原因。
        reason: &'static str,
    },
}

impl From<toml::de::Error> for RuleParseError {
    fn from(source: toml::de::Error) -> Self {
        Self::Toml {
            source: Box::new(source),
        }
    }
}

impl From<ModelError> for RuleParseError {
    fn from(source: ModelError) -> Self {
        Self::Model {
            source: Box::new(source),
        }
    }
}
