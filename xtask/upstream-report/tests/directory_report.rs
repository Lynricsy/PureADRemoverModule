#![doc = "目录输入 report-only 行为测试。"]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

#[test]
fn report_writes_manual_review_manifest_when_directory_contains_special_paths()
-> Result<(), Box<dyn Error>> {
    // Given: a local upstream directory with script material and JSON-hostile path bytes.
    let fixture = Fixture::new()?;
    fixture.write(
        Path::new("sdk cache/quote\"slash\\control\tfile.db"),
        "sqlite /data/user/0/pkg/cache/pangle\n",
    )?;
    fixture.write(
        Path::new("scripts/cleanup.sh"),
        "#!/system/bin/sh\nrm -rf TTCache\n",
    )?;
    let manifest = fixture.path("manifest.json");

    // When: the report-only tool scans the directory through the real CLI surface.
    let output = run_report([
        "--from-local",
        fixture.root_str(),
        "--report-only",
        "--manifest",
        path_str(&manifest)?,
    ])?;

    // Then: the manifest is valid JSON and every candidate is manual-review-only.
    assert_success(&output)?;
    let document = parse_json_file(&manifest)?;
    assert_policy(&document)?;
    assert_eq!(json_field(&document, "input")?["kind"], "directory");
    assert_all_auto_import_false(&document)?;

    let accepted = json_array_field(&document, "accepted")?;
    assert!(accepted.iter().any(|record| {
        json_string_field(record, "path")
            .is_some_and(|path| path.contains("quote\"slash\\control\tfile.db"))
    }));
    assert!(accepted.iter().any(|record| {
        json_string_field_eq(record, "review_state", "manual_review_only")
            && json_bool_field(record, "executable_upstream_code") == Some(true)
    }));
    Ok(())
}

fn assert_policy(document: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    let policy = json_field(document, "policy")?;
    assert_eq!(policy["report_only"], true);
    assert_eq!(policy["rules_modified"], false);
    assert_eq!(policy["download_performed"], false);
    assert_eq!(policy["snapshots_modified"], false);
    Ok(())
}

fn assert_all_auto_import_false(document: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    for field in ["accepted", "rejected", "ignored"] {
        for record in json_array_field(document, field)? {
            assert_eq!(json_bool_field(record, "auto_import_allowed"), Some(false));
        }
    }
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
        fs::create_dir(&root)?;
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
    let dir = std::env::temp_dir().join(format!(
        "puread-upstream-report-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir(&dir)?;
    Ok(dir)
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

fn json_string_field_eq(document: &Value, key: &str, expected: &str) -> bool {
    json_string_field(document, key) == Some(expected)
}

fn json_bool_field(document: &Value, key: &str) -> Option<bool> {
    json_field(document, key).ok()?.as_bool()
}
