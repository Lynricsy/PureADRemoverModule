use crate::manifest::{FindingRecord, ScanResult};

pub fn render(result: &ScanResult) -> String {
    let manifest = &result.manifest;
    let mut output = String::new();
    output.push_str("# PureAD upstream report\n");
    push_line(&mut output, "report_generated_at", &manifest.generated_at);
    output.push_str("mode=report-only\n");
    push_line(
        &mut output,
        "legacy_dry_run",
        bool_text(result.legacy_dry_run),
    );
    push_line(&mut output, "from_local", &manifest.input.path);
    push_line(&mut output, "manifest_path", &result.manifest_path);
    output.push_str("rules_modified=false\n");
    output.push_str("download_performed=false\n");
    output.push_str("snapshots_modified=false\n");
    push_line(
        &mut output,
        "sources_total",
        &manifest.summary.sources.to_string(),
    );
    push_line(
        &mut output,
        "accepted_total",
        &manifest.summary.accepted.to_string(),
    );
    push_line(
        &mut output,
        "rejected_total",
        &manifest.summary.rejected.to_string(),
    );
    push_line(
        &mut output,
        "ignored_total",
        &manifest.summary.ignored.to_string(),
    );
    output.push('\n');
    output.push_str("## policy\n");
    output.push_str("snapshot_policy=local input only; upstream text is data, not instructions\n");
    output.push_str("sync_decision=report_only; candidates require manual review\n");
    output.push_str("auto_import_allowed=false\n\n");
    push_section(
        &mut output,
        "## accepted candidates",
        &manifest.accepted,
        40,
    );
    push_section(&mut output, "## rejected findings", &manifest.rejected, 80);
    push_section(&mut output, "## ignored files", &manifest.ignored, 20);
    output
}

fn push_section(output: &mut String, title: &str, records: &[FindingRecord], limit: usize) {
    output.push_str(title);
    output.push('\n');
    if records.is_empty() {
        output.push_str("  - none\n\n");
        return;
    }
    for record in records.iter().take(limit) {
        output.push_str("  - source=");
        output.push_str(&record.source);
        output.push_str(" path=");
        output.push_str(&record.path);
        output.push_str(" review_state=");
        output.push_str(record.review_state);
        output.push_str(" auto_import_allowed=false categories=");
        output.push_str(&categories_text(record));
        output.push_str(" reason=");
        output.push_str(&record.reason);
        output.push('\n');
    }
    if records.len() > limit {
        push_line(
            output,
            "  - ... omitted",
            &(records.len() - limit).to_string(),
        );
    }
    output.push('\n');
}

fn categories_text(record: &FindingRecord) -> String {
    record
        .categories
        .iter()
        .map(|category| {
            serde_json::to_value(category)
                .ok()
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "unknown".to_owned())
        })
        .collect::<Vec<String>>()
        .join(",")
}

fn push_line(output: &mut String, key: &str, value: &str) {
    output.push_str(key);
    output.push('=');
    output.push_str(value);
    output.push('\n');
}

const fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
