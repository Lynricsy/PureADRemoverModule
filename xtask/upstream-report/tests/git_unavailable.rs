#![doc = "`git` 不可用时的目录来源报告测试。"]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

#[test]
fn report_writes_file_snapshot_when_git_binary_is_unavailable() -> Result<(), Box<dyn Error>> {
    // Given: a local upstream directory and a PATH that cannot resolve git.
    let fixture = Fixture::new()?;
    fixture.write(Path::new("ads-cache.db"), "sqlite TTCache\n")?;
    fs::create_dir(fixture.path("empty-bin"))?;
    let manifest = fixture.path("manifest.json");

    // When: the report-only tool scans through the real CLI surface without git available.
    let output = run_report_with_path(
        [
            "--from-local",
            fixture.root_str(),
            "--report-only",
            "--manifest",
            path_str(&manifest)?,
        ],
        path_str(&fixture.path("empty-bin"))?,
    )?;

    // Then: file snapshot reporting still succeeds and git-only fields degrade to n/a.
    assert_success(&output)?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("commit=n/a"), "{stdout}");
    let document = parse_json_file(&manifest)?;
    let source = json_array_field(&document, "sources")?
        .first()
        .ok_or("missing source record")?;
    assert_eq!(json_string_field(source, "remote_url"), Some("n/a"));
    assert_eq!(json_string_field(source, "commit"), Some("n/a"));
    assert_eq!(json_string_field(source, "sha256"), Some("n/a"));
    assert!(!json_array_field(&document, "accepted")?.is_empty());
    Ok(())
}

struct Fixture {
    temp_dir: PathBuf,
    root: PathBuf,
    root_display: String,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let temp_dir = unique_temp_dir()?;
        let root = temp_dir.join("upstream");
        fs::create_dir_all(&root)?;
        let root_display = path_str(&root)?.to_owned();
        Ok(Self {
            temp_dir,
            root,
            root_display,
        })
    }

    fn root_str(&self) -> &str {
        &self.root_display
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.temp_dir.join(relative)
    }

    fn write(&self, relative: &Path, content: &str) -> Result<(), Box<dyn Error>> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _cleanup_result = fs::remove_dir_all(&self.temp_dir);
    }
}

fn run_report_with_path<const N: usize>(
    args: [&str; N],
    path: &str,
) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_puread-upstream-report"))
        .args(args)
        .env("PATH", path)
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

fn json_string_field<'a>(document: &'a Value, key: &str) -> Option<&'a str> {
    json_field(document, key).ok()?.as_str()
}
