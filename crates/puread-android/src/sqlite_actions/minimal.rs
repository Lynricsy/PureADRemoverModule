use std::path::Path;

use crate::sqlite_actions::error::SqliteActionError;

const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";
const PAGE_SIZE: usize = 4096;
const HEADER_SIZE: usize = 100;
const LEAF_TABLE_PAGE: u8 = 0x0d;

pub(super) fn minimal_sqlite_image(path: &Path) -> Result<Vec<u8>, SqliteActionError> {
    let mut image = vec![0_u8; PAGE_SIZE];
    write_bytes(&mut image, 0, SQLITE_MAGIC, path)?;
    write_u16(&mut image, 16, PAGE_SIZE_U16, path)?;
    write_byte(&mut image, 18, 1, path)?;
    write_byte(&mut image, 19, 1, path)?;
    write_byte(&mut image, 20, 0, path)?;
    write_byte(&mut image, 21, 64, path)?;
    write_byte(&mut image, 22, 32, path)?;
    write_byte(&mut image, 23, 32, path)?;
    write_u32(&mut image, 24, 1, path)?;
    write_u32(&mut image, 28, 1, path)?;
    write_u32(&mut image, 44, 4, path)?;
    write_u32(&mut image, 56, 1, path)?;
    write_u32(&mut image, 92, 1, path)?;
    write_u32(&mut image, 96, 3_046_001, path)?;
    write_byte(&mut image, HEADER_SIZE, LEAF_TABLE_PAGE, path)?;
    write_u16(&mut image, 101, 0, path)?;
    write_u16(&mut image, 103, 0, path)?;
    write_u16(&mut image, 105, PAGE_SIZE_U16, path)?;
    validate_sqlite_image(path, &image)?;
    Ok(image)
}

pub(super) fn validate_sqlite_image(path: &Path, image: &[u8]) -> Result<(), SqliteActionError> {
    if image.len() < PAGE_SIZE {
        return integrity(path, "image shorter than one sqlite page");
    }
    if image.get(..SQLITE_MAGIC.len()) != Some(SQLITE_MAGIC.as_slice()) {
        return integrity(path, "missing sqlite magic");
    }
    let page_size = read_u16(image, 16, path)?;
    if usize::from(page_size) != PAGE_SIZE {
        return integrity(path, "unsupported sqlite page size");
    }
    if image.get(18) != Some(&1) || image.get(19) != Some(&1) {
        return integrity(path, "unsupported sqlite write/read version");
    }
    if image.get(21) != Some(&64) || image.get(22) != Some(&32) || image.get(23) != Some(&32) {
        return integrity(path, "invalid sqlite payload fractions");
    }
    if read_u32(image, 28, path)? != 1 {
        return integrity(path, "sqlite page count must be one");
    }
    if !(1..=4).contains(&read_u32(image, 44, path)?) {
        return integrity(path, "invalid sqlite schema format");
    }
    validate_first_page(path, image)
}

fn validate_first_page(path: &Path, image: &[u8]) -> Result<(), SqliteActionError> {
    if image.get(HEADER_SIZE) != Some(&LEAF_TABLE_PAGE) {
        return integrity(path, "first page is not a table leaf page");
    }
    if read_u16(image, 103, path)? != 0 {
        return integrity(path, "minimal sqlite must contain zero cells");
    }
    if usize::from(read_u16(image, 105, path)?) != PAGE_SIZE {
        return integrity(path, "invalid first page content start");
    }
    Ok(())
}

const PAGE_SIZE_U16: u16 = 4096;

fn write_byte(
    image: &mut [u8],
    offset: usize,
    value: u8,
    path: &Path,
) -> Result<(), SqliteActionError> {
    let Some(slot) = image.get_mut(offset) else {
        return integrity(path, "sqlite byte field out of bounds");
    };
    *slot = value;
    Ok(())
}

fn write_bytes(
    image: &mut [u8],
    offset: usize,
    value: &[u8],
    path: &Path,
) -> Result<(), SqliteActionError> {
    let Some(slot) = image.get_mut(offset..offset.saturating_add(value.len())) else {
        return integrity(path, "sqlite bytes field out of bounds");
    };
    slot.copy_from_slice(value);
    Ok(())
}

fn write_u16(
    image: &mut [u8],
    offset: usize,
    value: u16,
    path: &Path,
) -> Result<(), SqliteActionError> {
    write_bytes(image, offset, &value.to_be_bytes(), path)
}

fn write_u32(
    image: &mut [u8],
    offset: usize,
    value: u32,
    path: &Path,
) -> Result<(), SqliteActionError> {
    write_bytes(image, offset, &value.to_be_bytes(), path)
}

fn read_u16(image: &[u8], offset: usize, path: &Path) -> Result<u16, SqliteActionError> {
    let Some(bytes) = image.get(offset..offset.saturating_add(2)) else {
        return integrity(path, "sqlite u16 field out of bounds");
    };
    let bytes = bytes.try_into().map_err(|_| SqliteActionError::Integrity {
        path: path.to_path_buf(),
        reason: "sqlite u16 field size mismatch",
    })?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_u32(image: &[u8], offset: usize, path: &Path) -> Result<u32, SqliteActionError> {
    let Some(bytes) = image.get(offset..offset.saturating_add(4)) else {
        return integrity(path, "sqlite u32 field out of bounds");
    };
    let bytes = bytes.try_into().map_err(|_| SqliteActionError::Integrity {
        path: path.to_path_buf(),
        reason: "sqlite u32 field size mismatch",
    })?;
    Ok(u32::from_be_bytes(bytes))
}

fn integrity<T>(path: &Path, reason: &'static str) -> Result<T, SqliteActionError> {
    Err(SqliteActionError::Integrity {
        path: path.to_path_buf(),
        reason,
    })
}
