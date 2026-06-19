use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token, Waker};

use crate::{
    DaemonError, DaemonEvent, EventLoopConfig, FileRuleDaemonConfig, FileRuleDaemonRuntime,
};

#[path = "event_loop/control.rs"]
mod control;
#[path = "watcher.rs"]
mod watcher;

use control::{ControlHandle, ControlMessage};
use watcher::InotifyWatcher;

const INOTIFY_TOKEN: Token = Token(0);
const CONTROL_TOKEN: Token = Token(1);
const INOTIFY_BUFFER_SIZE: usize = 4096;
const EPOLL_CAPACITY: usize = 16;

struct DebouncedBatch {
    paths: Vec<PathBuf>,
    controls: Vec<ControlMessage>,
}

/// daemon 文件 watcher 事件循环。
#[derive(Debug)]
pub struct EventLoop {
    receiver: mpsc::Receiver<ControlMessage>,
    poll: Poll,
    events: Events,
    watcher: InotifyWatcher,
    debounce: Duration,
    file_rule_runtime: Option<FileRuleDaemonRuntime>,
}

/// daemon 事件循环控制句柄。
#[derive(Debug, Clone)]
pub struct EventLoopHandle {
    control: ControlHandle,
}

impl EventLoop {
    /// 创建事件循环并注册 watch roots。
    pub fn new(config: EventLoopConfig) -> Result<(Self, EventLoopHandle), DaemonError> {
        let (watch_roots, debounce) = config.into_parts();
        let (sender, receiver) = mpsc::channel();
        let watcher = InotifyWatcher::new(&watch_roots)?;
        let poll = Poll::new().map_err(|source| DaemonError::PollCreate { source })?;
        let raw_fd = watcher.raw_fd();
        let mut source = SourceFd(&raw_fd);
        poll.registry()
            .register(&mut source, INOTIFY_TOKEN, Interest::READABLE)
            .map_err(|source| DaemonError::PollRegister { source })?;
        let waker = Waker::new(poll.registry(), CONTROL_TOKEN)
            .map_err(|source| DaemonError::PollRegister { source })?;
        let event_loop = Self {
            receiver,
            poll,
            events: Events::with_capacity(EPOLL_CAPACITY),
            watcher,
            debounce,
            file_rule_runtime: None,
        };
        let handle = EventLoopHandle {
            control: ControlHandle::new(sender, std::sync::Arc::new(waker)),
        };
        Ok((event_loop, handle))
    }

    /// 创建 dry-run 文件规则 daemon 事件循环。
    pub fn from_file_rules(
        config: &FileRuleDaemonConfig,
    ) -> Result<(Self, EventLoopHandle), DaemonError> {
        let runtime = config.prepare()?;
        Self::from_file_rule_runtime(runtime)
    }

    /// 从已准备好的文件规则运行态创建事件循环。
    pub fn from_file_rule_runtime(
        runtime: FileRuleDaemonRuntime,
    ) -> Result<(Self, EventLoopHandle), DaemonError> {
        let (mut event_loop, handle) = Self::new(EventLoopConfig::new(
            runtime.watch_roots().to_vec(),
            runtime.debounce(),
        )?)?;
        event_loop.file_rule_runtime = Some(runtime);
        Ok((event_loop, handle))
    }

    /// 阻塞运行事件循环，直到收到 shutdown signal。
    pub fn run(
        &mut self,
        mut emit: impl FnMut(DaemonEvent) -> Result<(), DaemonError>,
    ) -> Result<(), DaemonError> {
        emit(DaemonEvent::Started)?;
        let mut running = true;
        while running {
            self.wait(None)?;
            running = self.dispatch_ready_events(&mut emit)?;
        }
        Ok(())
    }

    fn dispatch_ready_events(
        &mut self,
        emit: &mut impl FnMut(DaemonEvent) -> Result<(), DaemonError>,
    ) -> Result<bool, DaemonError> {
        let mut running = true;
        let mut saw_inotify = false;
        let mut saw_control = false;
        for event in &self.events {
            match event.token() {
                INOTIFY_TOKEN => saw_inotify = true,
                CONTROL_TOKEN => saw_control = true,
                _other => {}
            }
        }
        if saw_inotify {
            let batch = self.collect_debounced()?;
            if !batch.paths.is_empty() {
                let paths = batch.paths;
                emit(DaemonEvent::FilesChanged {
                    paths: paths.clone(),
                })?;
                self.emit_file_rule_effect(&paths, emit)?;
            }
            if !batch.controls.is_empty() {
                running = control::emit_controls(batch.controls, emit)?;
            }
        }
        if saw_control {
            let control_running = self.drain_control(emit)?;
            running = running && control_running;
        }
        Ok(running)
    }

    fn collect_debounced(&mut self) -> Result<DebouncedBatch, DaemonError> {
        let mut paths = self.read_inotify_paths()?;
        let mut controls = Vec::new();
        let mut collecting = true;
        while collecting {
            self.wait(Some(self.debounce))?;
            let mut saw_inotify = false;
            let mut saw_control = false;
            for event in &self.events {
                match event.token() {
                    INOTIFY_TOKEN => saw_inotify = true,
                    CONTROL_TOKEN => saw_control = true,
                    _other => {}
                }
            }
            if saw_inotify {
                add_unique_paths(self.read_inotify_paths()?, &mut paths);
            }
            if saw_control {
                control::collect_controls(&self.receiver, &mut controls);
                collecting = false;
            }
            if !saw_inotify && !saw_control {
                collecting = false;
            }
        }
        Ok(DebouncedBatch { paths, controls })
    }

    fn wait(&mut self, timeout: Option<Duration>) -> Result<(), DaemonError> {
        self.events.clear();
        self.poll
            .poll(&mut self.events, timeout)
            .map_err(|source| DaemonError::PollWait { source })
    }

    fn read_inotify_paths(&mut self) -> Result<Vec<PathBuf>, DaemonError> {
        self.watcher.read_paths(INOTIFY_BUFFER_SIZE)
    }

    fn emit_file_rule_effect(
        &self,
        paths: &[PathBuf],
        emit: &mut impl FnMut(DaemonEvent) -> Result<(), DaemonError>,
    ) -> Result<(), DaemonError> {
        let Some(runtime) = &self.file_rule_runtime else {
            return Ok(());
        };
        if runtime.is_apply_mode() {
            let outcomes = runtime.apply_for_paths(paths)?;
            if outcomes.is_empty() {
                return Ok(());
            }
            return emit(DaemonEvent::FileRuleApplyReport { outcomes });
        }
        let actions = runtime.dry_run_for_paths(paths)?;
        if actions.is_empty() {
            return Ok(());
        }
        emit(DaemonEvent::DryRunFilePlan { actions })
    }

    fn drain_control(
        &self,
        emit: &mut impl FnMut(DaemonEvent) -> Result<(), DaemonError>,
    ) -> Result<bool, DaemonError> {
        let mut controls = Vec::new();
        control::collect_controls(&self.receiver, &mut controls);
        control::emit_controls(controls, emit)
    }
}

impl EventLoopHandle {
    /// 请求事件循环重新加载配置。
    pub fn reload(&self) -> Result<(), DaemonError> {
        self.control.reload()
    }

    /// 请求事件循环停止。
    pub fn shutdown(&self) -> Result<(), DaemonError> {
        self.control.shutdown()
    }
}

fn add_unique_paths(new_paths: Vec<PathBuf>, paths: &mut Vec<PathBuf>) {
    for path in new_paths {
        push_unique(path, paths);
    }
}

fn push_unique(path: PathBuf, paths: &mut Vec<PathBuf>) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}
