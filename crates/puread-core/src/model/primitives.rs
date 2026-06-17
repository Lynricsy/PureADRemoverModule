use std::path::{Component, Path};

use crate::error::ModelError;

const ROOT_PATH_FIELD: &str = "root_path";
const RULE_ID_FIELD: &str = "rule_id";
const PACKAGE_NAME_FIELD: &str = "package_name";
const RESTORE_TOKEN_FIELD: &str = "restore_token";
const MAX_RULE_ID_LEN: usize = 96;
const MAX_RESTORE_TOKEN_LEN: usize = 128;

/// 类型化规则标识。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleId(String);

impl RuleId {
    /// 解析规则标识。
    pub fn parse(raw: &str) -> Result<Self, ModelError> {
        validate_ascii_token(RULE_ID_FIELD, raw, MAX_RULE_ID_LEN).map(Self)
    }

    /// 返回原始规则标识。
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Android 包名。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageName(String);

impl PackageName {
    /// 解析 Android 包名。
    pub fn parse(raw: &str) -> Result<Self, ModelError> {
        if raw.is_empty() {
            return Err(ModelError::Empty {
                field: PACKAGE_NAME_FIELD,
            });
        }

        let mut segment_count = 0_usize;
        for segment in raw.split('.') {
            if !is_valid_package_segment(segment) {
                return Err(ModelError::InvalidFormat {
                    field: PACKAGE_NAME_FIELD,
                    value: raw.to_owned(),
                });
            }
            segment_count = segment_count.saturating_add(1);
        }

        if segment_count < 2 {
            return Err(ModelError::InvalidFormat {
                field: PACKAGE_NAME_FIELD,
                value: raw.to_owned(),
            });
        }

        Ok(Self(raw.to_owned()))
    }

    /// 返回原始包名。
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// 已校验的 Android 根路径。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RootPath(String);

impl RootPath {
    /// 解析安全根路径。
    pub fn parse(raw: &str) -> Result<Self, ModelError> {
        if raw.is_empty() {
            return Err(ModelError::Empty {
                field: ROOT_PATH_FIELD,
            });
        }
        if !Path::new(raw).is_absolute() {
            return Err(ModelError::RelativeRootPath {
                value: raw.to_owned(),
            });
        }
        if raw.contains("//") || raw.contains('\0') {
            return Err(ModelError::InvalidFormat {
                field: ROOT_PATH_FIELD,
                value: raw.to_owned(),
            });
        }
        if has_parent_component(raw) {
            return Err(ModelError::EscapingRootPath {
                value: raw.to_owned(),
            });
        }

        let trimmed = raw.trim_end_matches('/');
        let normalized = if trimmed.is_empty() { "/" } else { trimmed };
        if is_dangerous_root(normalized) {
            return Err(ModelError::DangerousRootPath {
                value: raw.to_owned(),
            });
        }

        Ok(Self(normalized.to_owned()))
    }

    /// 返回规范化路径字符串。
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// 恢复账本 token。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RestoreToken(String);

impl RestoreToken {
    /// 解析恢复账本 token。
    pub fn parse(raw: &str) -> Result<Self, ModelError> {
        validate_ascii_token(RESTORE_TOKEN_FIELD, raw, MAX_RESTORE_TOKEN_LEN).map(Self)
    }

    /// 返回原始 token。
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

fn validate_ascii_token(
    field: &'static str,
    raw: &str,
    max_len: usize,
) -> Result<String, ModelError> {
    if raw.is_empty() {
        return Err(ModelError::Empty { field });
    }
    if raw.len() > max_len {
        return Err(ModelError::InvalidFormat {
            field,
            value: raw.to_owned(),
        });
    }

    let Some(first) = raw.chars().next() else {
        return Err(ModelError::Empty { field });
    };
    if !is_token_start(first) {
        return Err(ModelError::InvalidFormat {
            field,
            value: raw.to_owned(),
        });
    }
    if raw.ends_with('-') || raw.ends_with('_') || raw.ends_with('.') {
        return Err(ModelError::InvalidFormat {
            field,
            value: raw.to_owned(),
        });
    }
    for ch in raw.chars().skip(1) {
        if !is_token_continue(ch) {
            return Err(ModelError::InvalidFormat {
                field,
                value: raw.to_owned(),
            });
        }
    }

    Ok(raw.to_owned())
}

const fn is_token_start(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit()
}

const fn is_token_continue(ch: char) -> bool {
    is_token_start(ch) || ch == '-' || ch == '_' || ch == '.'
}

fn is_valid_package_segment(segment: &str) -> bool {
    let Some(first) = segment.chars().next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    segment
        .chars()
        .skip(1)
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn has_parent_component(raw: &str) -> bool {
    Path::new(raw)
        .components()
        .any(|component| component == Component::ParentDir)
}

fn is_dangerous_root(root: &str) -> bool {
    matches!(
        root,
        "/" | "/data" | "/sdcard" | "/storage" | "/system" | "/vendor" | "/data/adb"
    )
}
