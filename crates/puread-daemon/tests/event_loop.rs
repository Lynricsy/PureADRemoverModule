#![doc = "`puread-daemon` 事件循环骨架集成测试。"]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use puread_daemon::{DaemonError, DaemonEvent, EventLoop, EventLoopConfig};

const EVENT_TIMEOUT: Duration = Duration::from_secs(3);
const DEBOUNCE: Duration = Duration::from_millis(40);
static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn event_loop_emits_debounced_file_event_when_file_is_created()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a daemon event loop watches a temporary directory.
    let temp = TestTempDir::new()?;
    let watched_path = temp.path().to_path_buf();
    let marker = watched_path.join("marker.txt");
    let (started_tx, started_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let config = EventLoopConfig::new(vec![watched_path], DEBOUNCE)?;
    let (mut event_loop, handle) = EventLoop::new(config)?;
    let worker = thread::spawn(move || {
        event_loop.run(|event| {
            if matches!(event, DaemonEvent::Started) {
                started_tx
                    .send(())
                    .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            }
            event_tx
                .send(event)
                .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            Ok(())
        })
    });

    started_rx.recv_timeout(EVENT_TIMEOUT)?;

    // When: a file appears in the watched directory.
    std::fs::write(&marker, "created by event-loop test")?;

    // Then: one debounced file event reports the created path.
    let event = recv_matching(&event_rx, is_file_event, EVENT_TIMEOUT)?;
    assert!(event_contains_path(&event, &marker));

    handle.shutdown()?;
    let run_result = worker_result(worker, EVENT_TIMEOUT)?;
    run_result?;
    Ok(())
}

#[test]
fn event_loop_drains_reload_and_shutdown_during_debounce() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: a file event starts the debounce window for a running event loop.
    let temp = TestTempDir::new()?;
    let watched_path = temp.path().to_path_buf();
    let marker = watched_path.join("marker-control.txt");
    let (started_tx, started_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let config = EventLoopConfig::new(vec![watched_path], Duration::from_millis(120))?;
    let (mut event_loop, handle) = EventLoop::new(config)?;
    let worker = thread::spawn(move || {
        event_loop.run(|event| {
            if matches!(event, DaemonEvent::Started) {
                started_tx
                    .send(())
                    .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            }
            event_tx
                .send(event)
                .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            Ok(())
        })
    });

    started_rx.recv_timeout(EVENT_TIMEOUT)?;

    // When: reload and shutdown arrive before the debounce window closes.
    fs_write(&marker)?;
    thread::sleep(Duration::from_millis(20));
    handle.reload()?;
    handle.shutdown()?;

    // Then: both controls are emitted and the loop exits.
    let reload = recv_matching(&event_rx, is_reload_event, EVENT_TIMEOUT)?;
    let shutdown = recv_matching(&event_rx, is_shutdown_event, EVENT_TIMEOUT)?;
    assert!(matches!(reload, DaemonEvent::ReloadRequested));
    assert!(matches!(shutdown, DaemonEvent::ShutdownRequested));
    let run_result = worker_result(worker, EVENT_TIMEOUT)?;
    run_result?;
    Ok(())
}

#[test]
fn event_loop_does_not_restart_after_shutdown_during_file_control_race()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: file and control events can be observed in the same dispatch pass.
    let temp = TestTempDir::new()?;
    let watched_path = temp.path().to_path_buf();
    let marker = watched_path.join("marker-race.txt");
    let (started_tx, started_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let config = EventLoopConfig::new(vec![watched_path], Duration::from_millis(120))?;
    let (mut event_loop, handle) = EventLoop::new(config)?;
    let worker = thread::spawn(move || {
        event_loop.run(|event| {
            if matches!(event, DaemonEvent::Started) {
                started_tx
                    .send(())
                    .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            }
            event_tx
                .send(event)
                .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            Ok(())
        })
    });

    started_rx.recv_timeout(EVENT_TIMEOUT)?;

    // When: a file event and shutdown are both ready before dispatch completes.
    fs_write(&marker)?;
    handle.shutdown()?;

    // Then: shutdown is emitted and the loop exits instead of being restarted.
    let shutdown = recv_matching(&event_rx, is_shutdown_event, EVENT_TIMEOUT)?;
    assert!(matches!(shutdown, DaemonEvent::ShutdownRequested));
    let run_result = worker_result(worker, EVENT_TIMEOUT)?;
    run_result?;
    Ok(())
}

#[test]
fn event_loop_emits_reload_event_when_reload_signal_arrives()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a daemon event loop has entered its blocking receive path.
    let temp = TestTempDir::new()?;
    let (started_tx, started_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let config = EventLoopConfig::new(vec![temp.path().to_path_buf()], DEBOUNCE)?;
    let (mut event_loop, handle) = EventLoop::new(config)?;
    let worker = thread::spawn(move || {
        event_loop.run(|event| {
            if matches!(event, DaemonEvent::Started) {
                started_tx
                    .send(())
                    .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            }
            event_tx
                .send(event)
                .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            Ok(())
        })
    });

    started_rx.recv_timeout(EVENT_TIMEOUT)?;

    // When: the reload signal is forwarded to the loop.
    handle.reload()?;

    // Then: the loop emits a reload event without stopping.
    let event = recv_matching(&event_rx, is_reload_event, EVENT_TIMEOUT)?;
    assert!(matches!(event, DaemonEvent::ReloadRequested));

    handle.shutdown()?;
    let run_result = worker_result(worker, EVENT_TIMEOUT)?;
    run_result?;
    Ok(())
}

#[test]
fn event_loop_stops_when_shutdown_signal_arrives() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a daemon event loop is running against a temporary watch root.
    let temp = TestTempDir::new()?;
    let (started_tx, started_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let config = EventLoopConfig::new(vec![temp.path().to_path_buf()], DEBOUNCE)?;
    let (mut event_loop, handle) = EventLoop::new(config)?;
    let worker = thread::spawn(move || {
        event_loop.run(|event| {
            if matches!(event, DaemonEvent::Started) {
                started_tx
                    .send(())
                    .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            }
            event_tx
                .send(event)
                .map_err(|_source| DaemonError::CallbackChannelClosed)?;
            Ok(())
        })
    });

    started_rx.recv_timeout(EVENT_TIMEOUT)?;

    // When: the shutdown signal is forwarded to the loop.
    handle.shutdown()?;

    // Then: the loop reports shutdown and returns.
    let event = recv_matching(&event_rx, is_shutdown_event, EVENT_TIMEOUT)?;
    assert!(matches!(event, DaemonEvent::ShutdownRequested));
    let run_result = worker_result(worker, EVENT_TIMEOUT)?;
    run_result?;
    Ok(())
}

fn recv_matching(
    receiver: &mpsc::Receiver<DaemonEvent>,
    predicate: fn(&DaemonEvent) -> bool,
    timeout: Duration,
) -> Result<DaemonEvent, Box<dyn std::error::Error>> {
    for _attempt in 0..16 {
        let event = receiver.recv_timeout(timeout)?;
        if predicate(&event) {
            return Ok(event);
        }
    }
    Err("matching daemon event was not observed".into())
}

fn worker_result(
    worker: thread::JoinHandle<Result<(), DaemonError>>,
    timeout: Duration,
) -> Result<Result<(), DaemonError>, Box<dyn std::error::Error>> {
    let (done_tx, done_rx) = mpsc::channel();
    thread::spawn(move || {
        let result = match worker.join() {
            Ok(run_result) => run_result,
            Err(_panic) => Err(DaemonError::WorkerPanicked),
        };
        let _ignored = done_tx.send(result);
    });
    Ok(done_rx.recv_timeout(timeout)?)
}

const fn is_file_event(event: &DaemonEvent) -> bool {
    matches!(event, DaemonEvent::FilesChanged { .. })
}

const fn is_reload_event(event: &DaemonEvent) -> bool {
    matches!(event, DaemonEvent::ReloadRequested)
}

const fn is_shutdown_event(event: &DaemonEvent) -> bool {
    matches!(event, DaemonEvent::ShutdownRequested)
}

fn event_contains_path(event: &DaemonEvent, expected: &Path) -> bool {
    let DaemonEvent::FilesChanged { paths } = event else {
        return false;
    };
    paths.iter().any(|path| same_path(path, expected))
}

fn same_path(left: &PathBuf, right: &Path) -> bool {
    left == right
}

fn fs_write(path: &Path) -> std::io::Result<()> {
    std::fs::write(path, "created by event-loop test")
}

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new() -> Result<Self, std::io::Error> {
        let base = std::env::temp_dir();
        let process_id = std::process::id();
        for _attempt in 0..128 {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::SeqCst);
            let path = base.join(format!("puread-daemon-event-loop-{process_id}-{id}"));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not create unique daemon event-loop temp directory",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.path));
    }
}
