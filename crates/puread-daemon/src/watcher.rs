use std::collections::HashMap;
use std::os::fd::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};

use inotify::{Inotify, WatchDescriptor, WatchMask};

use crate::DaemonError;

#[derive(Debug)]
pub(super) struct InotifyWatcher {
    inotify: Inotify,
    watch_roots: HashMap<WatchDescriptor, PathBuf>,
}

impl InotifyWatcher {
    pub(super) fn new(roots: &[PathBuf]) -> Result<Self, DaemonError> {
        let inotify = Inotify::init().map_err(|source| DaemonError::InotifyCreate { source })?;
        let watch_roots = register_watch_roots(&inotify, roots)?;
        Ok(Self {
            inotify,
            watch_roots,
        })
    }

    pub(super) fn raw_fd(&self) -> RawFd {
        self.inotify.as_raw_fd()
    }

    pub(super) fn read_paths(&mut self, buffer_size: usize) -> Result<Vec<PathBuf>, DaemonError> {
        let mut buffer = vec![0_u8; buffer_size];
        let events = self
            .inotify
            .read_events(&mut buffer)
            .map_err(|source| DaemonError::InotifyRead { source })?;
        let mut paths = Vec::new();
        for event in events {
            if let Some(root) = self.watch_roots.get(&event.wd) {
                let path = event
                    .name
                    .map_or_else(|| root.clone(), |name| root.join(name));
                push_unique(path, &mut paths);
            }
        }
        Ok(paths)
    }
}

fn register_watch_roots(
    inotify: &Inotify,
    roots: &[PathBuf],
) -> Result<HashMap<WatchDescriptor, PathBuf>, DaemonError> {
    let mut watched = HashMap::new();
    for root in roots {
        ensure_root_exists(root)?;
        let descriptor = inotify
            .watches()
            .add(root, watch_mask())
            .map_err(|source| DaemonError::WatchPath {
                path: root.clone(),
                source,
            })?;
        watched.insert(descriptor, root.clone());
    }
    Ok(watched)
}

fn ensure_root_exists(path: &Path) -> Result<(), DaemonError> {
    if path.exists() {
        return Ok(());
    }
    Err(DaemonError::WatchRootMissing {
        path: path.to_path_buf(),
    })
}

fn watch_mask() -> WatchMask {
    WatchMask::CREATE
        | WatchMask::MODIFY
        | WatchMask::DELETE
        | WatchMask::MOVED_FROM
        | WatchMask::MOVED_TO
        | WatchMask::ATTRIB
}

fn push_unique(path: PathBuf, paths: &mut Vec<PathBuf>) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}
