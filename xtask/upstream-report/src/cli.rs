use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "puread-upstream-report",
    version,
    about = "PureAD 上游 report-only 分类工具"
)]
pub struct Cli {
    #[arg(long, default_value = "Example")]
    pub from_local: PathBuf,
    #[arg(long)]
    pub report_only: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long, default_value = "upstream/upstream_manifest.json")]
    pub manifest: PathBuf,
}

impl Cli {
    pub const fn legacy_dry_run(&self) -> bool {
        self.dry_run
    }

    pub const fn mode_enabled(&self) -> bool {
        self.report_only || self.dry_run
    }
}
