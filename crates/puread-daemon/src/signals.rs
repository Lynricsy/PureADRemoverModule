use std::fmt;

use crate::{DaemonError, EventLoopHandle};

/// Unix signal 到 daemon 控制事件的转发器。
#[cfg(unix)]
pub struct SignalForwarder {
    handle: signal_hook::iterator::Handle,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(unix)]
impl SignalForwarder {
    /// 监听 SIGINT/SIGTERM 作为 shutdown，监听 SIGHUP 作为 reload。
    pub fn start(event_loop: EventLoopHandle) -> Result<Self, DaemonError> {
        use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGTERM};
        use signal_hook::iterator::Signals;

        let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP])
            .map_err(|source| DaemonError::SignalInstall { source })?;
        let handle = signals.handle();
        let worker = std::thread::spawn(move || {
            for signal in signals.forever() {
                match signal {
                    SIGINT | SIGTERM => {
                        drop(event_loop.shutdown());
                        break;
                    }
                    SIGHUP => drop(event_loop.reload()),
                    _other => {}
                }
            }
        });
        Ok(Self {
            handle,
            worker: Some(worker),
        })
    }
}

#[cfg(unix)]
impl Drop for SignalForwarder {
    fn drop(&mut self) {
        self.handle.close();
        if let Some(worker) = self.worker.take() {
            drop(worker.join());
        }
    }
}

#[cfg(unix)]
impl fmt::Debug for SignalForwarder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignalForwarder")
            .finish_non_exhaustive()
    }
}

/// 非 Unix 平台的 signal 转发占位类型。
#[cfg(not(unix))]
#[derive(Debug)]
pub struct SignalForwarder;
