use std::error::Error;

use super::restore_adversarial;

#[test]
fn cli_restore_execute_rejects_target_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::target_symlink_is_not_followed()
}

#[test]
fn cli_restore_execute_rejects_target_parent_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::target_parent_symlink_is_not_followed()
}

#[test]
fn cli_restore_execute_rejects_directory_restore_parent_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::directory_restore_parent_symlink_is_not_followed()
}

#[test]
fn cli_restore_execute_rejects_remove_placeholder_parent_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::remove_placeholder_parent_symlink_is_not_followed()
}

#[test]
fn cli_restore_execute_rejects_backup_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::backup_symlink_is_not_read()
}

#[test]
fn cli_restore_execute_rejects_backup_parent_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::backup_parent_symlink_is_not_followed()
}

#[test]
fn cli_restore_execute_rejects_backup_path_escape() -> Result<(), Box<dyn Error>> {
    restore_adversarial::backup_path_escape_is_rejected()
}

#[test]
fn cli_restore_execute_recreates_missing_parent_directory_safely() -> Result<(), Box<dyn Error>> {
    restore_adversarial::missing_parent_directory_restore_uses_safe_path()
}

#[test]
fn cli_restore_execute_restores_permissions_safely() -> Result<(), Box<dyn Error>> {
    restore_adversarial::permission_restore_uses_safe_path()
}
