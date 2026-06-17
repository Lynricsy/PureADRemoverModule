use thiserror::Error;

/// 规则模型边界解析错误。
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelError {
    /// 输入字段为空。
    #[error("{field} must not be empty")]
    Empty {
        /// 出错字段名。
        field: &'static str,
    },

    /// 输入字段格式不符合模型约束。
    #[error("{field} has invalid format: {value}")]
    InvalidFormat {
        /// 出错字段名。
        field: &'static str,
        /// 原始输入值。
        value: String,
    },

    /// 输入枚举值不受支持。
    #[error("{field} has unsupported value: {value}")]
    UnsupportedValue {
        /// 出错字段名。
        field: &'static str,
        /// 原始输入值。
        value: String,
    },

    /// 路径不是绝对 Android 路径。
    #[error("root path must be absolute: {value}")]
    RelativeRootPath {
        /// 原始输入路径。
        value: String,
    },

    /// 路径包含父目录逃逸。
    #[error("root path must not contain parent directory traversal: {value}")]
    EscapingRootPath {
        /// 原始输入路径。
        value: String,
    },

    /// 路径指向过宽的危险根目录。
    #[error("root path is too broad or dangerous: {value}")]
    DangerousRootPath {
        /// 原始输入路径。
        value: String,
    },
}
