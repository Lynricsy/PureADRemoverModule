use std::path::PathBuf;
use std::sync::mpsc;

use puread_core::model::RuleAction;

/// daemon 事件循环错误。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DaemonError {
    /// watcher root 列表为空。
    #[error("event loop requires at least one watch root")]
    EmptyWatchRoots,

    /// 去抖间隔不能为零。
    #[error("event debounce interval must be greater than zero")]
    EmptyDebounce,

    /// watcher root 不存在。
    #[error("watch root does not exist: {path}")]
    WatchRootMissing {
        /// 缺失的 watch root。
        path: PathBuf,
    },

    /// 创建 inotify watcher 失败。
    #[error("failed to create inotify watcher")]
    InotifyCreate {
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 注册 watch root 失败。
    #[error("failed to watch path: {path}")]
    WatchPath {
        /// 注册失败的路径。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 读取 inotify 事件失败。
    #[error("failed to read inotify events")]
    InotifyRead {
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 创建 epoll 事件循环失败。
    #[error("failed to create epoll event loop")]
    PollCreate {
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 注册 epoll source 失败。
    #[error("failed to register epoll source")]
    PollRegister {
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 等待 epoll 事件失败。
    #[error("failed while waiting for daemon events")]
    PollWait {
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 唤醒 epoll 事件循环失败。
    #[error("failed to wake daemon event loop")]
    PollWake {
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// daemon 控制通道已经关闭。
    #[error("daemon control channel is closed")]
    ControlChannelClosed,

    /// daemon 事件通道已经关闭。
    #[error("daemon event channel is closed")]
    EventChannelClosed,

    /// 事件回调无法接收事件。
    #[error("daemon event callback channel is closed")]
    CallbackChannelClosed,

    /// 安装 Unix signal handler 失败。
    #[cfg(unix)]
    #[error("failed to install unix signal handler")]
    SignalInstall {
        /// signal-hook 返回的底层错误。
        #[source]
        source: std::io::Error,
    },

    /// daemon worker 线程异常退出。
    #[error("daemon worker thread aborted")]
    WorkerPanicked,

    /// 规则 root 列表为空。
    #[error("file-rule daemon requires at least one rule root")]
    EmptyRuleRoots,

    /// 文件规则动作不能进入 daemon 高频执行器。
    #[error("unsupported file action for daemon apply mode: {action:?}")]
    UnsupportedFileAction {
        /// 被拒绝的动作。
        action: RuleAction,
    },

    /// Android-like 根目录不存在。
    #[error("android root does not exist: {path}")]
    AndroidRootMissing {
        /// 缺失路径。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// Android-like 根路径不是目录。
    #[error("android root is not a directory: {path}")]
    AndroidRootNotDirectory {
        /// 非目录路径。
        path: PathBuf,
    },

    /// 规则 root 不存在。
    #[error("rule root does not exist: {path}")]
    RuleRootMissing {
        /// 缺失路径。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 规则 root 不是目录。
    #[error("rule root is not a directory: {path}")]
    RuleRootNotDirectory {
        /// 非目录路径。
        path: PathBuf,
    },

    /// 读取规则目录失败。
    #[error("failed to read rule directory: {path}")]
    RuleReadDir {
        /// 规则目录。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 读取规则文件元数据失败。
    #[error("failed to read rule metadata: {path}")]
    RuleMetadata {
        /// 规则路径。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 读取规则文件失败。
    #[error("failed to read rule file: {path}")]
    RuleRead {
        /// 规则路径。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },

    /// 解析规则文件失败。
    #[error("failed to parse rule file: {path}")]
    RuleParse {
        /// 规则路径。
        path: PathBuf,
        /// 规则解析错误。
        #[source]
        source: puread_rules::RuleParseError,
    },

    /// 展开文件规则路径失败。
    #[error("failed to expand file rule path for {rule_id}")]
    PathExpansion {
        /// 规则 ID。
        rule_id: String,
        /// 路径展开错误。
        #[source]
        source: puread_core::path_expansion::PathExpansionError,
    },

    /// 文件动作执行失败。
    #[error("file action execution failed")]
    FileAction {
        /// 底层文件动作错误。
        #[source]
        source: puread_android::file_actions::FileActionError,
    },
}

impl From<mpsc::RecvError> for DaemonError {
    fn from(_source: mpsc::RecvError) -> Self {
        Self::EventChannelClosed
    }
}
