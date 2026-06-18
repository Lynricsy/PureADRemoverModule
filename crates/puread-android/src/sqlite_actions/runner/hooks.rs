use std::path::Path;

#[cfg(test)]
type Hook = Box<dyn FnMut(&Path)>;
#[cfg(test)]
type HookCell = std::cell::RefCell<Option<Hook>>;

#[cfg(test)]
thread_local! {
    static AFTER_LEDGER_APPEND: HookCell = std::cell::RefCell::new(None);
    static AFTER_WRITE: HookCell = std::cell::RefCell::new(None);
    static BEFORE_WRITE_OPEN: HookCell = std::cell::RefCell::new(None);
    static BEFORE_DELETE: HookCell = std::cell::RefCell::new(None);
    static BEFORE_BACKUP: HookCell = std::cell::RefCell::new(None);
    static BEFORE_DELETE_MOVE: HookCell = std::cell::RefCell::new(None);
    static BEFORE_DISCARD_CLEANUP: HookCell = std::cell::RefCell::new(None);
}

#[cfg(test)]
fn run_hook(slot: &'static std::thread::LocalKey<HookCell>, path: &Path) {
    slot.with(|hook| {
        if let Some(callback) = hook.borrow_mut().as_mut() {
            callback(path);
        }
    });
}

#[cfg(test)]
fn with_hook<F, R>(slot: &'static std::thread::LocalKey<HookCell>, hook: Hook, run: F) -> R
where
    F: FnOnce() -> R,
{
    slot.with(|cell| {
        *cell.borrow_mut() = Some(hook);
    });
    let result = run();
    slot.with(|cell| {
        *cell.borrow_mut() = None;
    });
    result
}

#[cfg(test)]
pub(super) fn run_after_sqlite_ledger_append_for_tests(path: &Path) {
    run_hook(&AFTER_LEDGER_APPEND, path);
}

#[cfg(not(test))]
pub(super) const fn run_after_sqlite_ledger_append_for_tests(_path: &Path) {}

#[cfg(test)]
pub(super) fn run_after_sqlite_write_for_tests(path: &Path) {
    run_hook(&AFTER_WRITE, path);
}

#[cfg(not(test))]
pub(super) const fn run_after_sqlite_write_for_tests(_path: &Path) {}

#[cfg(test)]
pub(super) fn run_before_sqlite_write_open_for_tests(path: &Path) {
    run_hook(&BEFORE_WRITE_OPEN, path);
}

#[cfg(not(test))]
pub(super) const fn run_before_sqlite_write_open_for_tests(_path: &Path) {}

#[cfg(test)]
pub(super) fn run_before_sqlite_delete_for_tests(path: &Path) {
    run_hook(&BEFORE_DELETE, path);
}

#[cfg(not(test))]
pub(super) const fn run_before_sqlite_delete_for_tests(_path: &Path) {}

#[cfg(test)]
pub(super) fn run_before_sqlite_backup_for_tests(path: &Path) {
    run_hook(&BEFORE_BACKUP, path);
}

#[cfg(not(test))]
pub(super) const fn run_before_sqlite_backup_for_tests(_path: &Path) {}

#[cfg(test)]
pub(super) fn run_before_sqlite_delete_move_for_tests(path: &Path) {
    run_hook(&BEFORE_DELETE_MOVE, path);
}

#[cfg(not(test))]
pub(super) const fn run_before_sqlite_delete_move_for_tests(_path: &Path) {}

#[cfg(test)]
pub(super) fn run_before_sqlite_discard_cleanup_for_tests(path: &Path) {
    run_hook(&BEFORE_DISCARD_CLEANUP, path);
}

#[cfg(not(test))]
pub(super) const fn run_before_sqlite_discard_cleanup_for_tests(_path: &Path) {}

#[cfg(test)]
pub(super) fn with_after_sqlite_ledger_append_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    with_hook(&AFTER_LEDGER_APPEND, Box::new(hook), run)
}

#[cfg(test)]
pub(super) fn with_before_sqlite_write_open_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    with_hook(&BEFORE_WRITE_OPEN, Box::new(hook), run)
}

#[cfg(test)]
pub(super) fn with_after_sqlite_write_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    with_hook(&AFTER_WRITE, Box::new(hook), run)
}

#[cfg(test)]
pub(super) fn with_before_sqlite_delete_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    with_hook(&BEFORE_DELETE, Box::new(hook), run)
}

#[cfg(test)]
pub(super) fn with_before_sqlite_backup_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    with_hook(&BEFORE_BACKUP, Box::new(hook), run)
}

#[cfg(test)]
pub(super) fn with_before_sqlite_delete_move_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    with_hook(&BEFORE_DELETE_MOVE, Box::new(hook), run)
}

#[cfg(test)]
pub(super) fn with_before_sqlite_discard_cleanup_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    with_hook(&BEFORE_DISCARD_CLEANUP, Box::new(hook), run)
}
