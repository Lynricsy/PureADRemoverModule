use std::fs::File;
use std::io;
use std::path::Path;

#[cfg(not(unix))]
#[path = "secure_fs/fallback.rs"]
mod imp;
#[cfg(unix)]
#[path = "secure_fs/unix.rs"]
mod imp;

pub(crate) fn create_dir_all_no_symlink(path: &Path) -> io::Result<()> {
    imp::create_dir_all_no_symlink(path)
}

pub(crate) fn create_new_no_follow(path: &Path) -> io::Result<File> {
    imp::create_new_no_follow(path)
}

pub(crate) fn open_read_no_follow(path: &Path) -> io::Result<File> {
    imp::open_read_no_follow(path)
}

pub(crate) fn open_read_write_no_follow(path: &Path) -> io::Result<File> {
    imp::open_read_write_no_follow(path)
}

pub(crate) fn open_create_new_read_write_no_follow(path: &Path) -> io::Result<File> {
    imp::open_create_new_read_write_no_follow(path)
}

pub(crate) fn rename_no_symlink(from: &Path, to: &Path) -> io::Result<()> {
    imp::rename_no_symlink(from, to)
}

pub(crate) fn rename_new_no_symlink(from: &Path, to: &Path) -> io::Result<()> {
    imp::rename_new_no_symlink(from, to)
}

pub(crate) fn remove_file_no_symlink(path: &Path) -> io::Result<()> {
    imp::remove_file_no_symlink(path)
}

pub(crate) fn ensure_parent_no_symlink(path: &Path) -> io::Result<()> {
    imp::ensure_parent_no_symlink(path)
}

pub(crate) fn ensure_parent_beneath_root_no_symlink(
    path: &Path,
    filesystem_root: &Path,
) -> io::Result<()> {
    imp::ensure_parent_beneath_root_no_symlink(path, filesystem_root)
}

fn parent_path(path: &Path) -> io::Result<&Path> {
    path.parent()
        .ok_or_else(|| invalid_input("path has no parent"))
}

fn invalid_input(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
