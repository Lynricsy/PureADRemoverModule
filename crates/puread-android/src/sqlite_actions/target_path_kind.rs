use std::path::{Component, Path};

use crate::sqlite_actions::error::SqliteActionError;

pub(super) fn validate_android_database_path(path: &Path) -> Result<(), SqliteActionError> {
    if !path.is_absolute() || path.as_os_str().is_empty() {
        return invalid(path, "android sqlite path must be absolute");
    }
    if has_bad_component(path) {
        return invalid(path, "android sqlite path must not contain traversal");
    }
    if is_protected_android_root(path) || path_is_protected_android_subtree(path) {
        return invalid(path, "protected sqlite target rejected");
    }
    if !has_database_extension(path) {
        return invalid(path, "android sqlite path must use a database extension");
    }
    if !is_app_database_path(path) {
        return invalid(path, "android sqlite path must be an app database path");
    }
    Ok(())
}

pub(super) fn is_host_test_app_database_path(path: &Path) -> bool {
    let components = normal_components(path);
    components.windows(4).any(matches_app_database_window)
        || components.windows(6).any(matches_user_database_window)
}

pub(super) fn has_bad_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::CurDir | Component::Prefix(_)
        )
    })
}

pub(super) fn has_database_extension(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(is_database_extension)
}

pub(super) fn invalid<T>(path: &Path, reason: &'static str) -> Result<T, SqliteActionError> {
    Err(SqliteActionError::InvalidTarget {
        path: path.to_path_buf(),
        reason,
    })
}

fn is_app_database_path(path: &Path) -> bool {
    let components = normal_components(path);
    matches_data_data_database(&components) || matches_data_user_database(&components)
}

fn matches_data_data_database(components: &[String]) -> bool {
    match components {
        [data_root, data_user, package, databases, file]
            if data_root == "data"
                && data_user == "data"
                && is_package_name(package)
                && databases == "databases"
                && is_database_file(file) =>
        {
            true
        }
        _other => false,
    }
}

fn matches_data_user_database(components: &[String]) -> bool {
    match components {
        [data_root, user_root, user, package, databases, file]
            if data_root == "data"
                && user_root == "user"
                && is_decimal(user)
                && is_package_name(package)
                && databases == "databases"
                && is_database_file(file) =>
        {
            true
        }
        _other => false,
    }
}

fn matches_app_database_window(window: &[String]) -> bool {
    match window {
        [data_root, package, databases, file]
            if data_root == "data"
                && is_package_name(package)
                && databases == "databases"
                && is_database_file(file) =>
        {
            true
        }
        _other => false,
    }
}

fn matches_user_database_window(window: &[String]) -> bool {
    match window {
        [data_root, user_root, user, package, databases, file]
            if data_root == "data"
                && user_root == "user"
                && is_decimal(user)
                && is_package_name(package)
                && databases == "databases"
                && is_database_file(file) =>
        {
            true
        }
        _other => false,
    }
}

fn normal_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            Component::RootDir
            | Component::CurDir
            | Component::ParentDir
            | Component::Prefix(_) => None,
        })
        .collect()
}

fn is_database_file(value: &str) -> bool {
    Path::new(value)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(is_database_extension)
}

fn is_database_extension(extension: &str) -> bool {
    extension.eq_ignore_ascii_case("db")
        || extension.eq_ignore_ascii_case("sqlite")
        || extension.eq_ignore_ascii_case("sqlite3")
}

fn is_package_name(value: &str) -> bool {
    let mut count = 0_usize;
    for segment in value.split('.') {
        if segment.is_empty() || !segment.chars().all(is_package_char) {
            return false;
        }
        count = count.saturating_add(1);
    }
    count >= 2
}

const fn is_package_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_decimal(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
}

fn is_protected_android_root(path: &Path) -> bool {
    ["/", "/data", "/sdcard", "/storage", "/system", "/vendor"]
        .iter()
        .any(|protected| path == Path::new(protected))
}

fn path_is_protected_android_subtree(path: &Path) -> bool {
    ["/data/adb", "/data/local/tmp"]
        .iter()
        .any(|protected| path == Path::new(protected) || path.starts_with(protected))
}
