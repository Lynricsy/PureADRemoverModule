#!/bin/sh
set -eu

DEFAULT_INPUT="Example"
DEFAULT_MANIFEST="upstream/upstream_manifest.json"
TOOL_MANIFEST="xtask/upstream-report/Cargo.toml"

usage() {
    cat <<'EOF'
Usage:
  scripts/update-upstream.sh --from-local PATH --report-only
  scripts/update-upstream.sh --dry-run [--from-local PATH]

Create a report-only upstream audit from a local directory or zip file.
The tool never downloads upstream files, rewrites rules, or extracts snapshots
into the repository.
EOF
}

die() {
    printf '%s\n' "error: $*" >&2
    exit 1
}

unknown_arg() {
    printf '%s\n' "error: unknown argument: $1" >&2
    usage >&2
    exit 2
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

from_local=$DEFAULT_INPUT
manifest_path=$DEFAULT_MANIFEST
report_only=false
dry_run=false

if [ "$#" -eq 0 ]; then
    usage >&2
    exit 2
fi

while [ "$#" -gt 0 ]; do
    case "$1" in
        --from-local)
            shift
            [ "$#" -gt 0 ] || die "--from-local requires a path"
            from_local=$1
            ;;
        --manifest)
            shift
            [ "$#" -gt 0 ] || die "--manifest requires a path"
            manifest_path=$1
            ;;
        --report-only)
            report_only=true
            ;;
        --dry-run)
            dry_run=true
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            unknown_arg "$1"
            ;;
    esac
    shift
done

if [ "$report_only" != true ] && [ "$dry_run" != true ]; then
    die "use --report-only for local upstream audits"
fi

require_command cargo

set -- --from-local "$from_local" --manifest "$manifest_path"
if [ "$report_only" = true ]; then
    set -- "$@" --report-only
fi
if [ "$dry_run" = true ]; then
    set -- "$@" --dry-run
fi

cargo run --quiet --manifest-path "$TOOL_MANIFEST" -- "$@"
