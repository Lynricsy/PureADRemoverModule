use std::fs;
use std::path::Path;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::classifier::{Decision, classify, is_executable_upstream_code};
use crate::cli::Cli;
use crate::error::{
    ReportError, display_path, input_missing, io_at, unsupported_input, zip_not_file,
};
use crate::manifest::{
    DisabledFlag, EnabledFlag, FindingRecord, InputSummary, ItemKind, Manifest, Policy, ScanResult,
    SourceRecord, SourceType, Summary,
};

const REVIEW_STATE: &str = "manual_review_only";

mod fs_walk;
mod source_meta;
mod zip_read;

use fs_walk::{collect_files, is_zip_path, relative_display, sorted_children};
use source_meta::{git_value, sha256_bytes, sha256_file, source_id};
use zip_read::{zip_entries, zip_entry_content};

#[derive(Default)]
struct Accumulator {
    sources: Vec<SourceRecord>,
    accepted: Vec<FindingRecord>,
    rejected: Vec<FindingRecord>,
    ignored: Vec<FindingRecord>,
}

pub fn scan(args: &Cli) -> Result<ScanResult, ReportError> {
    if !args.mode_enabled() {
        return Err(ReportError::ReportOnlyRequired);
    }
    let input = args.from_local.as_path();
    ensure_input_exists(input)?;
    let generated_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|source| ReportError::TimeFormat { source })?;
    let mut acc = Accumulator::default();
    let input_kind = scan_input(input, &mut acc)?;
    let summary = Summary {
        sources: acc.sources.len(),
        accepted: acc.accepted.len(),
        rejected: acc.rejected.len(),
        ignored: acc.ignored.len(),
    };
    let manifest = Manifest {
        schema_version: 1,
        generated_at,
        mode: "report-only",
        input: InputSummary {
            path: display_path(input),
            kind: input_kind,
        },
        policy: report_only_policy(),
        summary,
        sources: acc.sources,
        accepted: acc.accepted,
        rejected: acc.rejected,
        ignored: acc.ignored,
    };
    write_manifest(args.manifest.as_path(), &manifest)?;
    Ok(ScanResult {
        manifest,
        legacy_dry_run: args.legacy_dry_run(),
        manifest_path: display_path(args.manifest.as_path()),
    })
}

fn scan_input(input: &Path, acc: &mut Accumulator) -> Result<&'static str, ReportError> {
    if is_zip_path(input) {
        let metadata = fs::metadata(input).map_err(|source| io_at(input, source))?;
        if !metadata.is_file() {
            return Err(zip_not_file(input));
        }
        scan_zip_source(input, &source_id(input), acc)?;
        return Ok("zip");
    }
    let metadata = fs::metadata(input).map_err(|source| io_at(input, source))?;
    if !metadata.is_dir() {
        return Err(unsupported_input(input));
    }
    scan_directory_input(input, acc)?;
    Ok("directory")
}

fn scan_directory_input(input: &Path, acc: &mut Accumulator) -> Result<(), ReportError> {
    let children = sorted_children(input)?;
    if children
        .iter()
        .any(|path| path.is_file() && !is_zip_path(path))
    {
        scan_directory_source(input, &source_id(input), acc)?;
    }
    for child in children {
        if child.is_file() && is_zip_path(&child) {
            scan_zip_source(&child, &source_id(&child), acc)?;
        } else if child.is_dir() {
            scan_directory_source(&child, &source_id(&child), acc)?;
        }
    }
    Ok(())
}

fn scan_directory_source(
    dir: &Path,
    source_id: &str,
    acc: &mut Accumulator,
) -> Result<(), ReportError> {
    let files = collect_files(dir)?;
    let executable = directory_has_executable_code(&files)?;
    let size_bytes = directory_size(&files)?;
    acc.sources.push(SourceRecord {
        source_id: source_id.to_owned(),
        source_type: SourceType::Directory,
        path: display_path(dir),
        sha256: "n/a".to_owned(),
        size_bytes,
        file_count: files.len(),
        remote_url: git_value(dir, &["remote", "get-url", "origin"])?,
        commit: git_value(dir, &["rev-parse", "HEAD"])?,
        executable_upstream_code: executable,
        auto_import_allowed: false,
    });
    for file in files {
        let content = fs::read(&file).map_err(|source| io_at(&file, source))?;
        let relative = relative_display(dir, &file);
        append_record(acc, source_id, ItemKind::File, &relative, None, &content);
    }
    Ok(())
}

fn scan_zip_source(
    zip_path: &Path,
    source_id: &str,
    acc: &mut Accumulator,
) -> Result<(), ReportError> {
    let entries = zip_entries(zip_path)?;
    let file_entries: Vec<String> = entries
        .into_iter()
        .filter(|entry| !entry.ends_with('/'))
        .collect();
    let executable = file_entries
        .iter()
        .any(|entry| is_executable_upstream_code(entry, &[]));
    acc.sources.push(SourceRecord {
        source_id: source_id.to_owned(),
        source_type: SourceType::Zip,
        path: display_path(zip_path),
        sha256: sha256_file(zip_path)?,
        size_bytes: fs::metadata(zip_path)
            .map_err(|source| io_at(zip_path, source))?
            .len(),
        file_count: file_entries.len(),
        remote_url: "n/a".to_owned(),
        commit: "n/a".to_owned(),
        executable_upstream_code: executable,
        auto_import_allowed: false,
    });
    for entry in file_entries {
        let content = zip_entry_content(zip_path, &entry)?;
        append_record(
            acc,
            source_id,
            ItemKind::ZipEntry,
            &entry,
            Some(entry.clone()),
            &content,
        );
    }
    Ok(())
}

fn append_record(
    acc: &mut Accumulator,
    source_id: &str,
    kind: ItemKind,
    path: &str,
    zip_entry: Option<String>,
    content: &[u8],
) {
    let classification = classify(path, content);
    let record = FindingRecord {
        source: source_id.to_owned(),
        kind,
        path: path.to_owned(),
        zip_entry,
        sha256: sha256_bytes(content),
        size_bytes: content_len(content),
        categories: classification.categories,
        signals: classification.signals,
        reason: classification.reason,
        review_state: REVIEW_STATE,
        auto_import_allowed: false,
        executable_upstream_code: is_executable_upstream_code(path, content),
    };
    match classification.decision {
        Decision::Accepted => acc.accepted.push(record),
        Decision::Rejected => acc.rejected.push(record),
        Decision::Ignored => acc.ignored.push(record),
    }
}

fn write_manifest(path: &Path, manifest: &Manifest) -> Result<(), ReportError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| io_at(parent, source))?;
    }
    let json =
        serde_json::to_vec_pretty(manifest).map_err(|source| ReportError::Json { source })?;
    fs::write(path, json).map_err(|source| io_at(path, source))
}

const fn report_only_policy() -> Policy {
    Policy {
        download_performed: DisabledFlag::False,
        rules_modified: DisabledFlag::False,
        snapshots_modified: DisabledFlag::False,
        report_only: EnabledFlag::True,
        auto_import_allowed: DisabledFlag::False,
    }
}

fn ensure_input_exists(input: &Path) -> Result<(), ReportError> {
    if input.exists() {
        return Ok(());
    }
    Err(input_missing(input))
}

fn directory_has_executable_code(files: &[std::path::PathBuf]) -> Result<bool, ReportError> {
    for path in files {
        let content = fs::read(path).map_err(|source| io_at(path, source))?;
        if is_executable_upstream_code(&display_path(path), &content) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn directory_size(files: &[std::path::PathBuf]) -> Result<u64, ReportError> {
    let mut total = 0_u64;
    for path in files {
        total += fs::metadata(path)
            .map_err(|source| io_at(path, source))?
            .len();
    }
    Ok(total)
}

fn content_len(content: &[u8]) -> u64 {
    u64::try_from(content.len()).unwrap_or(u64::MAX)
}
