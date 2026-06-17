use std::path::{Component, Path, PathBuf};

use super::PathExpansionError;

pub(super) fn validate_package(package: &str) -> Result<&str, PathExpansionError> {
    if package.trim() != package || !package.contains('.') {
        return Err(invalid_package(package));
    }
    for segment in package.split('.') {
        validate_package_segment(package, segment)?;
    }
    Ok(package)
}

pub(super) fn validate_segments(path: &str) -> Result<(), PathExpansionError> {
    if path
        .split('/')
        .any(|segment| segment == ".." || segment == ".")
    {
        return Err(PathExpansionError::TraversalSegment {
            path: path.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn validate_relative_segments(path: &str) -> Result<Vec<String>, PathExpansionError> {
    if path.trim().is_empty() {
        return Err(PathExpansionError::EmptyPath);
    }
    let relative = Path::new(path);
    if relative.is_absolute() {
        return Err(PathExpansionError::RelativePath {
            path: relative.to_path_buf(),
        });
    }
    validate_segments(path)?;
    if has_wildcard(path) || path.split('/').any(str::is_empty) {
        return Err(PathExpansionError::UnsupportedTemplate {
            template: path.to_owned(),
        });
    }
    Ok(path.split('/').map(ToOwned::to_owned).collect())
}

pub(super) fn validate_name(name: &str) -> Result<(), PathExpansionError> {
    if name.trim().is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || has_wildcard(name)
    {
        return Err(PathExpansionError::UnsafeName {
            name: name.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn belongs_to_package_scope(path: &Path, package: &str) -> bool {
    let mut segments = path.components().filter_map(|component| match component {
        Component::Normal(value) => value.to_str(),
        Component::RootDir | Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
            None
        }
    });
    match (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) {
        (Some("data"), Some("user"), Some(user), Some(pkg)) => {
            is_numeric_segment(user) && pkg == package
        }
        (Some("data"), Some("data"), Some(pkg), _)
        | (Some("sdcard"), Some("Android"), Some("data"), Some(pkg)) => pkg == package,
        _ => false,
    }
}

pub(super) fn is_protected_root(path: &Path) -> bool {
    [
        "/",
        "/data",
        "/sdcard",
        "/storage",
        "/system",
        "/vendor",
        "/data/adb",
    ]
    .iter()
    .any(|protected| path == Path::new(protected))
}

pub(super) fn has_wildcard(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[') || value.contains(']')
}

pub(super) fn has_root_wildcard(path: &str) -> bool {
    path.trim_start_matches('/')
        .split('/')
        .next()
        .is_some_and(has_wildcard)
}

pub(super) fn is_numeric_segment(segment: &str) -> bool {
    !segment.is_empty() && segment.chars().all(|ch| ch.is_ascii_digit())
}

pub(super) fn android_path_from_relative(relative: &Path) -> PathBuf {
    let mut android_path = PathBuf::from("/");
    android_path.push(relative);
    android_path
}

fn validate_package_segment(package: &str, segment: &str) -> Result<(), PathExpansionError> {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return Err(invalid_package(package));
    };
    if !first.is_ascii_lowercase()
        || chars.any(|ch| !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_')
    {
        return Err(invalid_package(package));
    }
    Ok(())
}

fn invalid_package(package: &str) -> PathExpansionError {
    PathExpansionError::InvalidPackage {
        package: package.to_owned(),
    }
}
