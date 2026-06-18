#![doc = "`file_rule_integration` 测试运行辅助。"]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use puread_daemon::{DaemonError, DaemonEvent};

static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

/// 接收第一个满足断言的 daemon 事件。
pub fn recv_matching(
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

/// 等待后台事件循环线程结束并返回其执行结果。
pub fn worker_result(
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

/// 自动清理的 daemon 集成测试临时目录。
#[derive(Debug)]
pub struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    /// 创建带进程号和自增后缀的临时目录。
    pub fn new(prefix: &str) -> Result<Self, std::io::Error> {
        let base = std::env::temp_dir();
        let process_id = std::process::id();
        for _attempt in 0..128 {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::SeqCst);
            let path = base.join(format!("{prefix}-{process_id}-{id}"));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not create unique daemon file-rule temp directory",
        ))
    }

    /// 返回临时目录根路径。
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.path));
    }
}
