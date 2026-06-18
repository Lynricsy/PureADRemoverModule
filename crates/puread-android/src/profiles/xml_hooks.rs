use std::path::Path;

#[cfg(test)]
pub(super) fn before_commit_open(path: &Path) -> Result<(), std::io::Error> {
    test_hooks::take_before_commit_open(path)
}

#[cfg(not(test))]
pub(super) fn before_commit_open(path: &Path) -> Result<(), std::io::Error> {
    if path.as_os_str().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty profile path",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod test_hooks {
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    type Hook = Box<dyn FnOnce(&Path) -> Result<(), std::io::Error> + Send>;

    static BEFORE_COMMIT_OPEN: OnceLock<Mutex<Option<Hook>>> = OnceLock::new();

    pub(in crate::profiles) fn set_before_commit_open(hook: Hook) {
        let lock = BEFORE_COMMIT_OPEN.get_or_init(|| Mutex::new(None));
        let mut guard = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(hook);
    }

    pub(super) fn take_before_commit_open(path: &Path) -> Result<(), std::io::Error> {
        let Some(lock) = BEFORE_COMMIT_OPEN.get() else {
            return Ok(());
        };
        let hook = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(hook) = hook {
            hook(path)?;
        }
        Ok(())
    }
}
