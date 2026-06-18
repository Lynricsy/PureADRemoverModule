mod guards;
mod metadata_ops;
mod primary;
#[cfg(test)]
mod test_hooks;

pub(super) use metadata_ops::apply_metadata_changes;
pub(super) use primary::apply_primary_action;

#[cfg(test)]
pub(super) use test_hooks::{
    with_after_file_helper_guard_hook_for_tests, with_before_file_delete_hook_for_tests,
    with_before_file_discard_cleanup_hook_for_tests, with_before_file_helper_guard_hook_for_tests,
    with_before_file_move_to_backup_hook_for_tests,
};
