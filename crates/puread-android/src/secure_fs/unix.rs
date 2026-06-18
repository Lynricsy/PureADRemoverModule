use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::{Component, Path};

use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, FileType, Mode, OFlags, RenameFlags, mkdirat, open, openat, renameat, renameat_with,
    statat, unlinkat,
};

use super::{invalid_input, parent_path};

const DIR_MODE: Mode = Mode::from_raw_mode(0o755);
const FILE_MODE: Mode = Mode::from_raw_mode(0o644);

pub(super) fn create_dir_all_no_symlink(path: &Path) -> io::Result<()> {
    validate_absolute(path)?;
    let mut dir = open_root_dir()?;
    for component in normal_components(path) {
        dir = match open_child_dir(&dir, component) {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                create_child_dir(&dir, component)?;
                open_child_dir(&dir, component)?
            }
            Err(error) => return Err(error),
        };
    }
    Ok(())
}

pub(super) fn create_new_no_follow(path: &Path) -> io::Result<File> {
    open_final(path, OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL)
}

pub(super) fn open_read_no_follow(path: &Path) -> io::Result<File> {
    open_final(path, OFlags::RDONLY)
}

pub(super) fn open_read_write_no_follow(path: &Path) -> io::Result<File> {
    open_final(path, OFlags::RDWR)
}

pub(super) fn open_create_new_read_write_no_follow(path: &Path) -> io::Result<File> {
    open_final(path, OFlags::RDWR | OFlags::CREATE | OFlags::EXCL)
}

pub(super) fn rename_no_symlink(from: &Path, to: &Path) -> io::Result<()> {
    let (from_parent, from_name) = open_parent_dir(from)?;
    let (to_parent, to_name) = open_parent_dir(to)?;
    renameat(&from_parent, from_name, &to_parent, to_name).map_err(io::Error::from)
}

pub(super) fn rename_new_no_symlink(from: &Path, to: &Path) -> io::Result<()> {
    let (from_parent, from_name) = open_parent_dir(from)?;
    let (to_parent, to_name) = open_parent_dir(to)?;
    renameat_with(
        &from_parent,
        from_name,
        &to_parent,
        to_name,
        RenameFlags::NOREPLACE,
    )
    .map_err(io::Error::from)
}

pub(super) fn remove_file_no_symlink(path: &Path) -> io::Result<()> {
    let (parent, name) = open_parent_dir(path)?;
    unlinkat(&parent, name, AtFlags::empty()).map_err(io::Error::from)
}

pub(super) fn ensure_parent_no_symlink(path: &Path) -> io::Result<()> {
    let _parent = open_dir_path(parent_path(path)?)?;
    Ok(())
}

pub(super) fn ensure_parent_beneath_root_no_symlink(
    path: &Path,
    filesystem_root: &Path,
) -> io::Result<()> {
    validate_absolute(path)?;
    validate_absolute(filesystem_root)?;
    let parent = parent_path(path)?;
    validate_absolute(parent)?;
    let relative_parent = parent
        .strip_prefix(filesystem_root)
        .map_err(|_error| invalid_input("parent directory escaped filesystem root"))?;
    let mut dir = open_dir_path(filesystem_root)?;
    for component in normal_components(relative_parent) {
        match stat_child(&dir, component)? {
            ChildKind::Directory => dir = open_child_dir(&dir, component)?,
            ChildKind::Missing => return Err(invalid_input("parent directory is missing")),
            ChildKind::Symlink => return Err(invalid_input("parent directory contains symlink")),
            ChildKind::Other => return Err(invalid_input("parent path contains non-directory")),
        }
    }
    Ok(())
}

fn open_final(path: &Path, flags: OFlags) -> io::Result<File> {
    let (parent, name) = open_parent_dir(path)?;
    let fd = openat(&parent, name, file_flags(flags), FILE_MODE).map_err(io::Error::from)?;
    Ok(File::from(fd))
}

fn open_parent_dir(path: &Path) -> io::Result<(OwnedFd, &OsStr)> {
    validate_absolute(path)?;
    let name = path
        .file_name()
        .ok_or_else(|| invalid_input("path has no final component"))?;
    Ok((open_dir_path(parent_path(path)?)?, name))
}

fn open_dir_path(path: &Path) -> io::Result<OwnedFd> {
    validate_absolute(path)?;
    let mut dir = open_root_dir()?;
    for component in normal_components(path) {
        dir = open_child_dir(&dir, component)?;
    }
    Ok(dir)
}

fn open_root_dir() -> io::Result<OwnedFd> {
    open(Path::new("/"), dir_flags(), Mode::empty()).map_err(io::Error::from)
}

fn open_child_dir(parent: &OwnedFd, name: &OsStr) -> io::Result<OwnedFd> {
    openat(parent, name, dir_flags(), Mode::empty()).map_err(io::Error::from)
}

fn create_child_dir(parent: &OwnedFd, name: &OsStr) -> io::Result<()> {
    match mkdirat(parent, name, DIR_MODE) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn stat_child(parent: &OwnedFd, name: &OsStr) -> io::Result<ChildKind> {
    match statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => Ok(child_kind(FileType::from_raw_mode(stat.st_mode))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ChildKind::Missing),
        Err(error) => Err(error.into()),
    }
}

fn child_kind(file_type: FileType) -> ChildKind {
    if file_type.is_dir() {
        ChildKind::Directory
    } else if file_type.is_symlink() {
        ChildKind::Symlink
    } else {
        ChildKind::Other
    }
}

fn validate_absolute(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(invalid_input("path must be absolute"));
    }
    validate_components(path)
}

fn validate_components(path: &Path) -> io::Result<()> {
    for component in path.components() {
        match component {
            Component::RootDir | Component::Normal(_) => {}
            Component::Prefix(_) | Component::CurDir | Component::ParentDir => {
                return Err(invalid_input("path must not traverse"));
            }
        }
    }
    Ok(())
}

fn normal_components(path: &Path) -> impl Iterator<Item = &OsStr> {
    path.components().filter_map(|component| match component {
        Component::Normal(name) => Some(name),
        Component::RootDir | Component::Prefix(_) | Component::CurDir | Component::ParentDir => {
            None
        }
    })
}

fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

fn file_flags(flags: OFlags) -> OFlags {
    flags | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

enum ChildKind {
    Directory,
    Symlink,
    Other,
    Missing,
}
