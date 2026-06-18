use std::fs::File;
use std::io;
use std::path::Path;

pub(super) fn create_dir_all_no_symlink(path: &Path) -> io::Result<()> {
    Err(unsupported(path))
}

pub(super) fn create_new_no_follow(path: &Path) -> io::Result<File> {
    Err(unsupported(path))
}

pub(super) fn open_read_no_follow(path: &Path) -> io::Result<File> {
    Err(unsupported(path))
}

pub(super) fn open_read_write_no_follow(path: &Path) -> io::Result<File> {
    Err(unsupported(path))
}

pub(super) fn open_create_new_read_write_no_follow(path: &Path) -> io::Result<File> {
    Err(unsupported(path))
}

pub(super) fn rename_no_symlink(from: &Path, to: &Path) -> io::Result<()> {
    Err(unsupported_pair(from, to))
}

pub(super) fn rename_new_no_symlink(from: &Path, to: &Path) -> io::Result<()> {
    Err(unsupported_pair(from, to))
}

pub(super) fn remove_file_no_symlink(path: &Path) -> io::Result<()> {
    Err(unsupported(path))
}

pub(super) fn ensure_parent_no_symlink(path: &Path) -> io::Result<()> {
    Err(unsupported(path))
}

pub(super) fn ensure_parent_beneath_root_no_symlink(
    path: &Path,
    _filesystem_root: &Path,
) -> io::Result<()> {
    Err(unsupported(path))
}

fn unsupported(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "secure filesystem operations require unix: {}",
            path.display()
        ),
    )
}

fn unsupported_pair(from: &Path, to: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "secure filesystem rename requires unix: {} -> {}",
            from.display(),
            to.display()
        ),
    )
}
