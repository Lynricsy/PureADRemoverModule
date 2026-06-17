use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// 路径展开过程中的类型化错误。
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PathExpansionError {
    /// 输入路径为空。
    #[error("path must not be empty")]
    EmptyPath,
    /// Android 目标路径必须是绝对路径。
    #[error("path must be absolute: {path:?}")]
    RelativePath {
        /// 被拒绝的原始路径。
        path: PathBuf,
    },
    /// 包名没有通过 Android 包名边界解析。
    #[error("invalid package name: {package}")]
    InvalidPackage {
        /// 被拒绝的包名。
        package: String,
    },
    /// 路径包含父目录跳转。
    #[error("path must not contain parent or current directory segments: {path}")]
    TraversalSegment {
        /// 被拒绝的路径。
        path: String,
    },
    /// 路径是受保护根目录本身。
    #[error("protected root path rejected: {path:?}")]
    ProtectedRoot {
        /// 被拒绝的路径。
        path: PathBuf,
    },
    /// `/data/adb` 只能指向本模块目录内部。
    #[error("path under /data/adb is outside module dir: {path:?}")]
    DataAdbOutsideModule {
        /// 被拒绝的路径。
        path: PathBuf,
        /// 允许的模块目录。
        module_dir: PathBuf,
    },
    /// 第一层路径通配会扩大到系统根级删除面。
    #[error("root-level wildcard is rejected: {template}")]
    RootLevelWildcard {
        /// 被拒绝的模板。
        template: String,
    },
    /// 通配符不属于受控模板。
    #[error("unsupported wildcard template: {template}")]
    UnsupportedWildcard {
        /// 被拒绝的模板。
        template: String,
    },
    /// 模板不属于允许的包级路径空间。
    #[error("unsupported path template: {template}")]
    UnsupportedTemplate {
        /// 被拒绝的模板。
        template: String,
    },
    /// 名称匹配输入不是单一文件名。
    #[error("unsafe name match input: {name}")]
    UnsafeName {
        /// 被拒绝的名称。
        name: String,
    },
    /// 解析宿主文件系统时发生 I/O 错误。
    #[error("io error at {path:?}: {source}")]
    Io {
        /// 发生错误的路径。
        path: PathBuf,
        /// 原始 I/O 错误。
        #[source]
        source: io::Error,
    },
    /// 符号链接解析到了 fake Android 根目录之外。
    #[error("symlink escapes filesystem root: android={android_path:?} host={host_path:?}")]
    SymlinkEscape {
        /// Android 逻辑路径。
        android_path: PathBuf,
        /// 宿主映射路径。
        host_path: PathBuf,
        /// 允许的宿主根目录。
        root: PathBuf,
    },
}
