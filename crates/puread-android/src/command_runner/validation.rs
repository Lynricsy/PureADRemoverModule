use crate::command_runner::CommandError;

const CLI_META: [char; 12] = [';', '&', '|', '`', '$', '<', '>', '*', '?', '[', ']', '\\'];

pub(super) fn validate_package(field: &'static str, value: &str) -> Result<(), CommandError> {
    validate_common(field, value)?;
    if value.split('.').count() < 2 || !value.split('.').all(is_android_identifier) {
        return Err(CommandError::invalid_argument(
            field,
            value,
            "package must be dot-separated Android identifiers",
        ));
    }
    Ok(())
}

pub(super) fn validate_component(component: &str) -> Result<(), CommandError> {
    if component.is_empty()
        || component.contains(char::is_whitespace)
        || component.contains('\0')
        || component.starts_with('-')
        || component.contains("..")
    {
        return Err(CommandError::invalid_argument(
            "component",
            component,
            "component must be non-empty, shell-inert, and not path-traversing",
        ));
    }
    let Some((package, name)) = component.split_once('/') else {
        return Err(CommandError::invalid_argument(
            "component",
            component,
            "component must include package/name",
        ));
    };
    validate_package("component.package", package)?;
    if name.is_empty() {
        return Err(CommandError::invalid_argument(
            "component",
            component,
            "component name must be non-empty",
        ));
    }
    if name.starts_with('-') || name.contains("..") || !name.chars().all(is_component_char) {
        return Err(CommandError::invalid_argument(
            "component",
            component,
            "component name must be an Android class name",
        ));
    }
    Ok(())
}

pub(super) fn validate_appop(op: &str) -> Result<(), CommandError> {
    validate_common("appop", op)?;
    if op.chars().all(|ch| ch.is_ascii_uppercase() || ch == '_') {
        return Ok(());
    }
    Err(CommandError::invalid_argument(
        "appop",
        op,
        "appop must be an uppercase Android AppOps token",
    ))
}

pub(super) fn validate_property(property: &str) -> Result<(), CommandError> {
    validate_common("property", property)?;
    if property
        .split('.')
        .all(|part| !part.is_empty() && part.chars().all(is_property_char))
    {
        return Ok(());
    }
    Err(CommandError::invalid_argument(
        "property",
        property,
        "property must be a dot-separated Android property name",
    ))
}

pub(super) fn validate_simple_token(field: &'static str, value: &str) -> Result<(), CommandError> {
    validate_common(field, value)
}

fn validate_common(field: &'static str, value: &str) -> Result<(), CommandError> {
    if value.is_empty() || value.contains(char::is_whitespace) || value.contains('\0') {
        return Err(CommandError::invalid_argument(
            field,
            value,
            "token must be non-empty and contain no whitespace or NUL",
        ));
    }
    if value.starts_with('-') || value.contains("..") || value.contains(CLI_META) {
        return Err(CommandError::invalid_argument(
            field,
            value,
            "token must not be option-like, path-traversing, or contain CLI metacharacters",
        ));
    }
    Ok(())
}

fn is_android_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

const fn is_component_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '$')
}

const fn is_property_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}
