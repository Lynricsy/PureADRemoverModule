use std::path::Path;

type FileHelperGuardHook = Box<dyn FnMut(&Path)>;
type FileHelperGuardHookCell = std::cell::RefCell<Option<FileHelperGuardHook>>;

thread_local! {
    static BEFORE_FILE_HELPER_GUARD_HOOK: FileHelperGuardHookCell = std::cell::RefCell::new(None);
    static AFTER_FILE_HELPER_GUARD_HOOK: FileHelperGuardHookCell = std::cell::RefCell::new(None);
    static BEFORE_FILE_DELETE_HOOK: FileHelperGuardHookCell = std::cell::RefCell::new(None);
    static BEFORE_FILE_MOVE_TO_BACKUP_HOOK: FileHelperGuardHookCell = std::cell::RefCell::new(None);
    static BEFORE_FILE_DISCARD_CLEANUP_HOOK: FileHelperGuardHookCell =
        std::cell::RefCell::new(None);
}

pub(super) fn run_before_file_helper_guard(path: &Path) {
    BEFORE_FILE_HELPER_GUARD_HOOK.with(|hook| {
        if let Some(callback) = hook.borrow_mut().as_mut() {
            callback(path);
        }
    });
}

pub(super) fn run_after_file_helper_guard(path: &Path) {
    AFTER_FILE_HELPER_GUARD_HOOK.with(|hook| {
        if let Some(callback) = hook.borrow_mut().as_mut() {
            callback(path);
        }
    });
}

pub(super) fn run_before_file_delete(path: &Path) {
    BEFORE_FILE_DELETE_HOOK.with(|hook| {
        if let Some(callback) = hook.borrow_mut().as_mut() {
            callback(path);
        }
    });
}

pub(super) fn run_before_file_move_to_backup(path: &Path) {
    BEFORE_FILE_MOVE_TO_BACKUP_HOOK.with(|hook| {
        if let Some(callback) = hook.borrow_mut().as_mut() {
            callback(path);
        }
    });
}

pub(super) fn run_before_file_discard_cleanup(path: &Path) {
    BEFORE_FILE_DISCARD_CLEANUP_HOOK.with(|hook| {
        if let Some(callback) = hook.borrow_mut().as_mut() {
            callback(path);
        }
    });
}

pub(in crate::file_actions) fn with_before_file_helper_guard_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    BEFORE_FILE_HELPER_GUARD_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    let result = run();
    BEFORE_FILE_HELPER_GUARD_HOOK.with(|slot| {
        *slot.borrow_mut() = None;
    });
    result
}

pub(in crate::file_actions) fn with_after_file_helper_guard_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    AFTER_FILE_HELPER_GUARD_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    let result = run();
    AFTER_FILE_HELPER_GUARD_HOOK.with(|slot| {
        *slot.borrow_mut() = None;
    });
    result
}

pub(in crate::file_actions) fn with_before_file_delete_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    BEFORE_FILE_DELETE_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    let result = run();
    BEFORE_FILE_DELETE_HOOK.with(|slot| {
        *slot.borrow_mut() = None;
    });
    result
}

pub(in crate::file_actions) fn with_before_file_move_to_backup_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    BEFORE_FILE_MOVE_TO_BACKUP_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    let result = run();
    BEFORE_FILE_MOVE_TO_BACKUP_HOOK.with(|slot| {
        *slot.borrow_mut() = None;
    });
    result
}

pub(in crate::file_actions) fn with_before_file_discard_cleanup_hook_for_tests<F, R>(
    hook: impl FnMut(&Path) + 'static,
    run: F,
) -> R
where
    F: FnOnce() -> R,
{
    BEFORE_FILE_DISCARD_CLEANUP_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    let result = run();
    BEFORE_FILE_DISCARD_CLEANUP_HOOK.with(|slot| {
        *slot.borrow_mut() = None;
    });
    result
}
