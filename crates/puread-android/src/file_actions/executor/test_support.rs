use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEST_ROOT: AtomicUsize = AtomicUsize::new(0);

pub(super) fn temp_root() -> Result<PathBuf, Box<dyn Error>> {
    let id = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "puread-file-actions-guard-{}-{id}",
        std::process::id()
    ));
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

pub(super) fn parent(path: &Path) -> Result<&Path, Box<dyn Error>> {
    path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing parent").into()
    })
}
