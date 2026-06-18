use std::path::PathBuf;
use std::time::Duration;

use crate::DaemonError;

/// daemon 事件循环配置。
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EventLoopConfig {
    watch_roots: Vec<PathBuf>,
    debounce: Duration,
}

impl EventLoopConfig {
    /// 创建 watcher 配置。
    ///
    /// watch root 为空或去抖间隔为零时返回错误，避免启动无意义循环。
    pub fn new(
        watch_roots: impl IntoIterator<Item = PathBuf>,
        debounce: Duration,
    ) -> Result<Self, DaemonError> {
        let roots = watch_roots.into_iter().collect::<Vec<_>>();
        if roots.is_empty() {
            return Err(DaemonError::EmptyWatchRoots);
        }
        if debounce.is_zero() {
            return Err(DaemonError::EmptyDebounce);
        }
        Ok(Self {
            watch_roots: roots,
            debounce,
        })
    }

    pub(crate) fn into_parts(self) -> (Vec<PathBuf>, Duration) {
        (self.watch_roots, self.debounce)
    }
}
