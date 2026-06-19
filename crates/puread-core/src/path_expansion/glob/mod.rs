mod parent;
mod pattern;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use parent::{
    AnyDataUserPackageGlobParent, DataUserGlobParent, looks_like_package_name, validate_parent,
};
use pattern::LastSegmentPattern;

use super::PathExpansionError;
use super::resolved::{ExpandedPath, PathResolver};
use super::validation::{has_wildcard, is_numeric_segment, validate_segments};

#[derive(Debug, Clone, PartialEq, Eq)]
struct LastSegmentGlob {
    parent: PathBuf,
    pattern: LastSegmentPattern,
}

pub(super) fn expand_last_segment_glob(
    template: &str,
    package: &str,
    resolver: &PathResolver,
) -> Result<Option<Vec<ExpandedPath>>, PathExpansionError> {
    let Some(glob) = LastSegmentGlob::parse(template, package)? else {
        return Ok(None);
    };
    glob.expand(resolver, package).map(Some)
}

impl LastSegmentGlob {
    fn parse(template: &str, package: &str) -> Result<Option<Self>, PathExpansionError> {
        let concrete = template.replace("<pkg>", package);
        validate_segments(&concrete)?;
        let Some((parent, pattern)) = template.rsplit_once('/') else {
            return Ok(None);
        };
        let Some(pattern) = LastSegmentPattern::parse(pattern, template, has_wildcard(parent))?
        else {
            return Ok(None);
        };
        if parent.is_empty() {
            return Err(PathExpansionError::UnsupportedWildcard {
                template: template.to_owned(),
            });
        }
        let concrete_parent = concrete_parent(parent, package);
        validate_parent(&concrete_parent, package, template)?;
        Ok(Some(Self {
            parent: concrete_parent,
            pattern,
        }))
    }

    fn expand(
        &self,
        resolver: &PathResolver,
        package: &str,
    ) -> Result<Vec<ExpandedPath>, PathExpansionError> {
        if let Some(paths) = self.expand_any_data_user_package_glob(resolver, package)? {
            return Ok(paths);
        }
        if let Some(paths) = self.expand_data_user_glob(resolver, package)? {
            return Ok(paths);
        }
        self.expand_parent(resolver, &self.parent)
    }

    fn expand_any_data_user_package_glob(
        &self,
        resolver: &PathResolver,
        package: &str,
    ) -> Result<Option<Vec<ExpandedPath>>, PathExpansionError> {
        let Some(parts) = AnyDataUserPackageGlobParent::parse(&self.parent, package)? else {
            return Ok(None);
        };
        let users_dir = resolver.host_from_android(Path::new("/data/user"));
        let mut expanded = Vec::new();
        for user_entry in PathResolver::read_dir_or_empty(&users_dir)? {
            let user = user_entry.file_name();
            let Some(user_name) = user.to_str() else {
                continue;
            };
            if !is_numeric_segment(user_name) {
                continue;
            }
            let user_dir = users_dir.join(user_name);
            for package_entry in PathResolver::read_dir_or_empty(&user_dir)? {
                let package_name = package_entry.file_name();
                let Some(package_name) = package_name.to_str() else {
                    continue;
                };
                if !looks_like_package_name(package_name) {
                    continue;
                }
                let parent = parts.android_parent(user_name, package_name);
                expanded.extend(self.expand_parent(resolver, &parent)?);
            }
        }
        Ok(Some(expanded))
    }

    fn expand_data_user_glob(
        &self,
        resolver: &PathResolver,
        package: &str,
    ) -> Result<Option<Vec<ExpandedPath>>, PathExpansionError> {
        let Some(parts) = DataUserGlobParent::parse(&self.parent, package)? else {
            return Ok(None);
        };
        let users_dir = resolver.host_from_android(Path::new("/data/user"));
        let mut expanded = Vec::new();
        for entry in PathResolver::read_dir_or_empty(&users_dir)? {
            let user = entry.file_name();
            let Some(user_name) = user.to_str() else {
                continue;
            };
            if !is_numeric_segment(user_name) {
                continue;
            }
            let parent = parts.android_parent(user_name);
            expanded.extend(self.expand_parent(resolver, &parent)?);
        }
        Ok(Some(expanded))
    }

    fn expand_parent(
        &self,
        resolver: &PathResolver,
        parent_android: &Path,
    ) -> Result<Vec<ExpandedPath>, PathExpansionError> {
        let parent_host = resolver.host_from_android(parent_android);
        let mut expanded = Vec::new();
        for entry in PathResolver::read_dir_or_empty(&parent_host)? {
            if self.matches(entry.file_name().as_ref()) {
                let android_path = parent_android.join(entry.file_name());
                if let Some(path) = resolver.resolve_existing(&android_path)? {
                    expanded.push(path);
                }
            }
        }
        Ok(expanded)
    }

    fn matches(&self, name: &OsStr) -> bool {
        name.to_str()
            .is_some_and(|value| self.pattern.matches(value))
    }
}

fn concrete_parent(parent: &str, package: &str) -> PathBuf {
    PathBuf::from(parent.replace("<pkg>", package))
}
