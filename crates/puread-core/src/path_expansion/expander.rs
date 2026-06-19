use std::fs;
use std::path::{Path, PathBuf};

use super::PathExpansionError;
use super::glob::expand_last_segment_glob;
pub use super::resolved::ExpandedPath;
use super::resolved::PathResolver;
use super::template::DataUserWildcard;
use super::validation::{
    belongs_to_package_scope, has_root_wildcard, has_wildcard, is_numeric_segment,
    is_protected_root, validate_name, validate_package, validate_relative_segments,
    validate_segments,
};

/// 只读路径展开器。
#[derive(Debug, Clone)]
pub struct PathExpander {
    module_android_dir: PathBuf,
    resolver: PathResolver,
}

impl PathExpander {
    /// 创建一个展开器。
    pub fn new(
        filesystem_root: impl Into<PathBuf>,
        module_android_dir: impl Into<PathBuf>,
    ) -> Result<Self, PathExpansionError> {
        let root = filesystem_root.into();
        let module_dir = module_android_dir.into();
        if module_dir.as_os_str().is_empty() {
            return Err(PathExpansionError::EmptyPath);
        }
        if !module_dir.is_absolute() {
            return Err(PathExpansionError::RelativePath { path: module_dir });
        }
        validate_segments(module_dir.to_string_lossy().as_ref())?;
        Ok(Self {
            resolver: PathResolver::new(root),
            module_android_dir: module_dir,
        })
    }

    /// 展开 Android 绝对路径模板。
    pub fn expand_template(
        &self,
        template: &str,
        package: &str,
    ) -> Result<Vec<ExpandedPath>, PathExpansionError> {
        let package = validate_package(package)?;
        self.validate_raw_template(template)?;
        if let Some(paths) = self.expand_last_segment_glob(template, package)? {
            return Ok(sort_dedup(paths));
        }
        if let Some(paths) = self.expand_data_user_wildcard(template, package)? {
            return Ok(sort_dedup(paths));
        }
        if has_wildcard(template) {
            return Err(PathExpansionError::UnsupportedWildcard {
                template: template.to_owned(),
            });
        }
        let concrete = template.replace("<pkg>", package);
        self.expand_concrete_template(&concrete, package)
    }

    /// 在受控包目录下展开相对路径。
    pub fn expand_package_relative(
        &self,
        package: &str,
        relative_path: &str,
    ) -> Result<Vec<ExpandedPath>, PathExpansionError> {
        let package = validate_package(package)?;
        let segments = validate_relative_segments(relative_path)?;
        let mut expanded = Vec::new();
        for root in self.package_roots(package)? {
            let mut android_path = root.android_path().to_path_buf();
            for segment in &segments {
                android_path.push(segment);
            }
            if let Some(path) = self.resolve_existing(&android_path)? {
                expanded.push(path);
            }
        }
        Ok(sort_dedup(expanded))
    }

    /// 在受控包目录下按文件名递归匹配。
    pub fn expand_name_match(
        &self,
        package: &str,
        name: &str,
    ) -> Result<Vec<ExpandedPath>, PathExpansionError> {
        let package = validate_package(package)?;
        validate_name(name)?;
        let mut expanded = Vec::new();
        for root in self.package_roots(package)? {
            self.walk_name_matches(root.host_path(), name, &mut expanded)?;
        }
        Ok(sort_dedup(expanded))
    }

    fn expand_concrete_template(
        &self,
        concrete: &str,
        package: &str,
    ) -> Result<Vec<ExpandedPath>, PathExpansionError> {
        self.validate_raw_template(concrete)?;
        let android_path = PathBuf::from(concrete);
        self.validate_allowed_concrete_path(&android_path, package, concrete)?;
        Ok(self
            .resolve_existing(&android_path)?
            .into_iter()
            .collect::<Vec<_>>())
    }

    fn validate_raw_template(&self, template: &str) -> Result<(), PathExpansionError> {
        if template.trim().is_empty() {
            return Err(PathExpansionError::EmptyPath);
        }
        validate_segments(template)?;
        let path = PathBuf::from(template);
        if !path.is_absolute() {
            return Err(PathExpansionError::RelativePath { path });
        }
        if has_root_wildcard(template) {
            return Err(PathExpansionError::RootLevelWildcard {
                template: template.to_owned(),
            });
        }
        if is_protected_root(&path) {
            return Err(PathExpansionError::ProtectedRoot { path });
        }
        if path.starts_with("/data/adb") && !path.starts_with(&self.module_android_dir) {
            return Err(PathExpansionError::DataAdbOutsideModule {
                path,
                module_dir: self.module_android_dir.clone(),
            });
        }
        Ok(())
    }

    fn validate_allowed_concrete_path(
        &self,
        path: &Path,
        package: &str,
        template: &str,
    ) -> Result<(), PathExpansionError> {
        if path.starts_with(&self.module_android_dir) || belongs_to_package_scope(path, package) {
            return Ok(());
        }
        Err(PathExpansionError::UnsupportedTemplate {
            template: template.to_owned(),
        })
    }

    fn expand_data_user_wildcard(
        &self,
        template: &str,
        package: &str,
    ) -> Result<Option<Vec<ExpandedPath>>, PathExpansionError> {
        let Some(parts) = DataUserWildcard::parse(template, package)? else {
            return Ok(None);
        };
        let users_dir = self.resolver.host_from_android(Path::new("/data/user"));
        let mut expanded = Vec::new();
        for entry in PathResolver::read_dir_or_empty(&users_dir)? {
            let user = entry.file_name();
            let Some(user_name) = user.to_str() else {
                continue;
            };
            if !is_numeric_segment(user_name) {
                continue;
            }
            let android_path = data_user_path(user_name, package, parts.suffix_segments());
            if let Some(path) = self.resolve_existing(&android_path)? {
                expanded.push(path);
            }
        }
        Ok(Some(expanded))
    }

    fn expand_last_segment_glob(
        &self,
        template: &str,
        package: &str,
    ) -> Result<Option<Vec<ExpandedPath>>, PathExpansionError> {
        let concrete = template.replace("<pkg>", package);
        expand_last_segment_glob(&concrete, package, &self.resolver)
    }

    fn package_roots(&self, package: &str) -> Result<Vec<ExpandedPath>, PathExpansionError> {
        let templates = [
            format!("/data/data/{package}"),
            format!("/sdcard/Android/data/{package}"),
        ];
        let mut roots = Vec::new();
        for template in templates {
            if let Some(path) = self.resolve_existing(Path::new(&template))? {
                roots.push(path);
            }
        }
        if let Some(paths) = self.expand_data_user_wildcard("/data/user/[0-9]*/<pkg>", package)? {
            roots.extend(paths);
        }
        Ok(sort_dedup(roots))
    }

    fn walk_name_matches(
        &self,
        root: &Path,
        name: &str,
        matches: &mut Vec<ExpandedPath>,
    ) -> Result<(), PathExpansionError> {
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in PathResolver::read_dir_or_empty(&dir)? {
                self.resolver.collect_name_match(&entry, name, matches)?;
                let host_path = entry.path();
                let metadata =
                    fs::symlink_metadata(&host_path).map_err(|source| PathExpansionError::Io {
                        path: host_path.clone(),
                        source,
                    })?;
                if metadata.file_type().is_dir() {
                    stack.push(host_path);
                }
            }
        }
        Ok(())
    }

    fn resolve_existing(
        &self,
        android_path: &Path,
    ) -> Result<Option<ExpandedPath>, PathExpansionError> {
        self.resolver.resolve_existing(android_path)
    }
}

fn data_user_path(user_name: &str, package: &str, suffix_segments: &[String]) -> PathBuf {
    let mut android_path = PathBuf::from("/data/user");
    android_path.push(user_name);
    android_path.push(package);
    for segment in suffix_segments {
        android_path.push(segment);
    }
    android_path
}

fn sort_dedup(mut paths: Vec<ExpandedPath>) -> Vec<ExpandedPath> {
    paths.sort_by(|left, right| left.android_path().cmp(right.android_path()));
    paths.dedup_by(|left, right| left.android_path() == right.android_path());
    paths
}
