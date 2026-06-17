use super::PathExpansionError;
use super::validation::has_wildcard;

#[derive(Debug)]
pub(super) struct DataUserWildcard {
    suffix_segments: Vec<String>,
}

impl DataUserWildcard {
    pub(super) fn parse(template: &str, package: &str) -> Result<Option<Self>, PathExpansionError> {
        let mut parts = template.trim_start_matches('/').split('/');
        let first = parts.next();
        let second = parts.next();
        let user_pattern = parts.next();
        let package_pattern = parts.next();
        if first != Some("data") || second != Some("user") {
            return Ok(None);
        }
        let Some(user_pattern) = user_pattern else {
            return Ok(None);
        };
        if !has_wildcard(user_pattern) {
            return Ok(None);
        }
        validate_user_pattern(template, user_pattern)?;
        validate_package_pattern(template, package_pattern, package)?;
        let suffix_segments = parts.map(ToOwned::to_owned).collect::<Vec<_>>();
        if suffix_segments.iter().any(|segment| has_wildcard(segment)) {
            return Err(PathExpansionError::UnsupportedWildcard {
                template: template.to_owned(),
            });
        }
        Ok(Some(Self { suffix_segments }))
    }

    pub(super) fn suffix_segments(&self) -> &[String] {
        &self.suffix_segments
    }
}

fn validate_user_pattern(template: &str, pattern: &str) -> Result<(), PathExpansionError> {
    if pattern == "*" || pattern == "[0-9]*" {
        return Ok(());
    }
    Err(PathExpansionError::UnsupportedWildcard {
        template: template.to_owned(),
    })
}

fn validate_package_pattern(
    template: &str,
    pattern: Option<&str>,
    package: &str,
) -> Result<(), PathExpansionError> {
    let Some(pattern) = pattern else {
        return Err(PathExpansionError::UnsupportedTemplate {
            template: template.to_owned(),
        });
    };
    if pattern == "<pkg>" || pattern == package {
        return Ok(());
    }
    Err(PathExpansionError::UnsupportedTemplate {
        template: template.to_owned(),
    })
}
