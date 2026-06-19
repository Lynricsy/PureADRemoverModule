use super::super::PathExpansionError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LastSegmentPattern {
    Exact(String),
    SingleGlob { prefix: String, suffix: String },
}

impl LastSegmentPattern {
    pub(super) fn parse(
        pattern: &str,
        template: &str,
        parent_has_wildcard: bool,
    ) -> Result<Option<Self>, PathExpansionError> {
        if unsupported_pattern(pattern) {
            return Err(PathExpansionError::UnsupportedWildcard {
                template: template.to_owned(),
            });
        }
        match pattern.matches('*').count() {
            0 if parent_has_wildcard => Ok(Some(Self::Exact(pattern.to_owned()))),
            0 => Ok(None),
            1 => {
                let (prefix, suffix) = pattern.split_once('*').ok_or_else(|| {
                    PathExpansionError::UnsupportedWildcard {
                        template: template.to_owned(),
                    }
                })?;
                Ok(Some(Self::SingleGlob {
                    prefix: prefix.to_owned(),
                    suffix: suffix.to_owned(),
                }))
            }
            _ => Err(PathExpansionError::UnsupportedWildcard {
                template: template.to_owned(),
            }),
        }
    }

    pub(super) fn matches(&self, name: &str) -> bool {
        match self {
            Self::Exact(expected) => name == expected,
            Self::SingleGlob { prefix, suffix } => {
                name.starts_with(prefix) && name.ends_with(suffix)
            }
        }
    }
}

fn unsupported_pattern(pattern: &str) -> bool {
    pattern.contains('?') || pattern.contains('[') || pattern.contains(']')
}
