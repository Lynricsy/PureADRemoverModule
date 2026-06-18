#![doc = "zip 输入 report-only 行为测试。"]

use std::error::Error;
use std::fs;
use std::num::TryFromIntError;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

#[test]
fn report_rejects_forbidden_zip_entry_without_extracting_to_repository()
-> Result<(), Box<dyn Error>> {
    // Given: a local zip snapshot with one forbidden host/domain entry.
    let fixture = Fixture::new()?;
    let zip_path = fixture.path("snapshot.zip");
    create_zip(
        &zip_path,
        &[("Host/ads_domain.txt", "0.0.0.0 gdt.qq.com\n")],
    )?;
    let manifest = fixture.path("zip-manifest.json");

    // When: the report-only tool scans the zip directly.
    let output = run_report([
        "--from-local",
        path_str(&zip_path)?,
        "--report-only",
        "--manifest",
        path_str(&manifest)?,
    ])?;

    // Then: the zip entry is rejected, surfaced as a zip entry, and no extraction dir appears.
    assert_success(&output)?;
    let document = parse_json_file(&manifest)?;
    assert_eq!(json_field(&document, "input")?["kind"], "zip");
    let rejected = json_array_field(&document, "rejected")?;
    assert!(rejected.iter().any(|record| {
        json_string_field_eq(record, "zip_entry", "Host/ads_domain.txt")
            && json_categories(record).contains(&"hosts".to_owned())
            && json_categories(record).contains(&"domain".to_owned())
    }));
    assert!(!fixture.path("Host").exists());
    Ok(())
}

fn json_categories(document: &Value) -> Vec<String> {
    json_field(document, "categories")
        .ok()
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

struct Fixture {
    temp_dir: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let temp_dir = unique_temp_dir()?;
        fs::create_dir(&temp_dir)?;
        Ok(Self { temp_dir })
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.temp_dir.join(relative)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _cleanup_result = fs::remove_dir_all(&self.temp_dir);
    }
}

fn run_report<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-upstream-report"))
        .args(args)
        .output()?)
}

fn unique_temp_dir() -> Result<PathBuf, Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string();
    Ok(std::env::temp_dir().join(format!(
        "puread-upstream-report-{}-{nanos}",
        std::process::id()
    )))
}

fn create_zip(path: &Path, entries: &[(&str, &str)]) -> Result<(), Box<dyn Error>> {
    let mut content = Vec::new();
    let mut central = Vec::new();
    for (name, body) in entries {
        append_stored_zip_entry(&mut content, &mut central, name, body.as_bytes())?;
    }
    let central_offset = u32::try_from(content.len())?;
    content.extend_from_slice(&central);
    append_eocd(
        &mut content,
        u16::try_from(entries.len())?,
        u32::try_from(central.len())?,
        central_offset,
    );
    fs::write(path, content)?;
    Ok(())
}

fn append_stored_zip_entry(
    output: &mut Vec<u8>,
    central: &mut Vec<u8>,
    name: &str,
    body: &[u8],
) -> Result<(), TryFromIntError> {
    let name_bytes = name.as_bytes();
    let crc = crc32(body);
    let size = u32::try_from(body.len())?;
    let name_len = u16::try_from(name_bytes.len())?;
    let local_offset = u32::try_from(output.len())?;
    push_local_header(output, crc, size, name_len);
    output.extend_from_slice(name_bytes);
    output.extend_from_slice(body);
    push_central_header(central, crc, size, name_len, local_offset);
    central.extend_from_slice(name_bytes);
    Ok(())
}

fn push_local_header(output: &mut Vec<u8>, crc: u32, size: u32, name_len: u16) {
    push_u32(output, 0x0403_4b50);
    push_u16(output, 20);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u32(output, crc);
    push_u32(output, size);
    push_u32(output, size);
    push_u16(output, name_len);
    push_u16(output, 0);
}

fn push_central_header(output: &mut Vec<u8>, crc: u32, size: u32, name_len: u16, offset: u32) {
    push_u32(output, 0x0201_4b50);
    push_u16(output, 20);
    push_u16(output, 20);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u32(output, crc);
    push_u32(output, size);
    push_u32(output, size);
    push_u16(output, name_len);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u32(output, 0);
    push_u32(output, offset);
}

fn append_eocd(output: &mut Vec<u8>, count: u16, central_size: u32, central_offset: u32) {
    push_u32(output, 0x0605_4b50);
    push_u16(output, 0);
    push_u16(output, 0);
    push_u16(output, count);
    push_u16(output, count);
    push_u32(output, central_size);
    push_u32(output, central_offset);
    push_u16(output, 0);
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _bit in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn path_str(path: &Path) -> Result<&str, Box<dyn Error>> {
    path.to_str()
        .ok_or_else(|| format!("path is not UTF-8: {}", path.display()).into())
}

fn assert_success(output: &Output) -> Result<(), Box<dyn Error>> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8(output.stderr.clone())?;
    Err(format!("CLI failed: {stderr}").into())
}

fn parse_json_file(path: &Path) -> Result<Value, Box<dyn Error>> {
    let content = fs::read(path)?;
    Ok(serde_json::from_slice(&content)?)
}

fn json_field<'a>(document: &'a Value, key: &str) -> Result<&'a Value, Box<dyn Error>> {
    document
        .get(key)
        .ok_or_else(|| format!("missing JSON field: {key}").into())
}

fn json_array_field<'a>(document: &'a Value, key: &str) -> Result<&'a [Value], Box<dyn Error>> {
    let Some(items) = json_field(document, key)?.as_array() else {
        return Err(format!("{key} must be a JSON array").into());
    };
    Ok(items)
}

fn json_string_field_eq(document: &Value, key: &str, expected: &str) -> bool {
    json_field(document, key).ok().and_then(Value::as_str) == Some(expected)
}
