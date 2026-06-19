use std::path::{Path, PathBuf};

use super::super::PathExpansionError;
use super::super::validation::{belongs_to_package_scope, has_wildcard};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AnyDataUserPackageGlobParent {
    suffix: Vec<String>,
}

impl AnyDataUserPackageGlobParent {
    pub(super) fn parse(parent: &Path, package: &str) -> Result<Option<Self>, PathExpansionError> {
        if package != "puread.sqlite.any" {
            return Ok(None);
        }
        let raw = parent.to_string_lossy();
        let mut parts = raw.trim_start_matches('/').split('/');
        if parts.next() != Some("data") || parts.next() != Some("user") {
            return Ok(None);
        }
        let Some(user_pattern) = parts.next() else {
            return Ok(None);
        };
        if !has_wildcard(user_pattern) {
            return Ok(None);
        }
        if user_pattern != "*" && user_pattern != "[0-9]*" {
            return Err(PathExpansionError::UnsupportedWildcard {
                template: raw.into_owned(),
            });
        }
        if parts.next() != Some("*") {
            return Ok(None);
        }
        Ok(Some(Self {
            suffix: parts.map(ToOwned::to_owned).collect(),
        }))
    }

    pub(super) fn android_parent(&self, user_name: &str, package_name: &str) -> PathBuf {
        let mut parent = PathBuf::from("/data/user");
        parent.push(user_name);
        parent.push(package_name);
        for segment in &self.suffix {
            parent.push(segment);
        }
        parent
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DataUserGlobParent {
    package: String,
    suffix: Vec<String>,
}

impl DataUserGlobParent {
    pub(super) fn parse(parent: &Path, package: &str) -> Result<Option<Self>, PathExpansionError> {
        let raw = parent.to_string_lossy();
        let mut parts = raw.trim_start_matches('/').split('/');
        if parts.next() != Some("data") || parts.next() != Some("user") {
            return Ok(None);
        }
        let Some(user_pattern) = parts.next() else {
            return Ok(None);
        };
        if !has_wildcard(user_pattern) {
            return Ok(None);
        }
        if user_pattern != "*" && user_pattern != "[0-9]*" {
            return Err(PathExpansionError::UnsupportedWildcard {
                template: raw.into_owned(),
            });
        }
        if parts.next() != Some(package) {
            return Err(PathExpansionError::UnsupportedTemplate {
                template: raw.into_owned(),
            });
        }
        Ok(Some(Self {
            package: package.to_owned(),
            suffix: parts.map(ToOwned::to_owned).collect(),
        }))
    }

    pub(super) fn android_parent(&self, user_name: &str) -> PathBuf {
        let mut parent = PathBuf::from("/data/user");
        parent.push(user_name);
        parent.push(&self.package);
        for segment in &self.suffix {
            parent.push(segment);
        }
        parent
    }
}

pub(super) fn validate_parent(
    parent: &Path,
    package: &str,
    template: &str,
) -> Result<(), PathExpansionError> {
    let parent_text = parent.to_string_lossy();
    if !has_wildcard(parent_text.as_ref()) && belongs_to_package_scope(parent, package) {
        return Ok(());
    }
    if AnyDataUserPackageGlobParent::parse(parent, package)?.is_some() {
        return Ok(());
    }
    if DataUserGlobParent::parse(parent, package)?.is_some() {
        return Ok(());
    }
    Err(PathExpansionError::UnsupportedWildcard {
        template: template.to_owned(),
    })
}

pub(super) fn looks_like_package_name(value: &str) -> bool {
    value.contains('.')
        && value
            .split('.')
            .all(|segment| !segment.is_empty() && segment.bytes().all(is_package_byte))
}

const fn is_package_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
}
