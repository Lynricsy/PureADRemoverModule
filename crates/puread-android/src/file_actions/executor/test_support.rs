use std::error::Error;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEST_ROOT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub(super) struct TestTempRoot {
    path: PathBuf,
}

impl TestTempRoot {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl AsRef<Path> for TestTempRoot {
    fn as_ref(&self) -> &Path {
        self.path.as_path()
    }
}

impl Deref for TestTempRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path.as_path()
    }
}

impl Drop for TestTempRoot {
    fn drop(&mut self) {
        let _ignored = std::fs::remove_dir_all(&self.path);
        remove_extension_siblings(self.path.as_path());
    }
}

pub(super) fn temp_root() -> Result<TestTempRoot, Box<dyn Error>> {
    let id = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "puread-file-actions-guard-{}-{id}",
        std::process::id()
    ));
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    std::fs::create_dir_all(&root)?;
    Ok(TestTempRoot::new(root))
}

pub(super) fn parent(path: &Path) -> Result<&Path, Box<dyn Error>> {
    path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing parent").into()
    })
}

fn remove_extension_siblings(path: &Path) {
    let Some(parent) = path.parent() else {
        return;
    };
    let Some(prefix) = path.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    let sibling_prefix = format!("{prefix}.");
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry_result in entries {
        let Ok(entry) = entry_result else {
            continue;
        };
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(sibling_prefix.as_str()) {
            continue;
        }
        let path = entry.path();
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                let _ignored = std::fs::remove_dir_all(path);
            }
            Ok(_metadata) => {
                let _ignored = std::fs::remove_file(path);
            }
            Err(_error) => {}
        }
    }
}
