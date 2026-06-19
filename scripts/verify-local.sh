#!/bin/sh
set -eu

EVIDENCE_DIR=".omo/evidence"
TASK_PREFIX="$EVIDENCE_DIR/task-26"

SCRIPT_DIR="$(dirname "$0")"
LIB_DIR="$SCRIPT_DIR/lib"

. "$LIB_DIR/verify-local-common.sh"
. "$LIB_DIR/verify-local-forbidden.sh"
. "$LIB_DIR/verify-local-quality.sh"
. "$LIB_DIR/verify-local-zip.sh"

if [ "${1-}" = "-h" ] || [ "${1-}" = "--help" ]; then
    usage
    exit 0
fi
if [ "${1-}" != "" ]; then
    usage >&2
    printf '%s\n' "error: unknown argument: $1" >&2
    exit 2
fi

if [ ! -f "AGENTS.md" ]; then
    printf '%s\n' "error: AGENTS.md is required at repository root" >&2
    exit 1
fi

cleanup_tmp_root() {
    [ -n "${TMP_ROOT:-}" ] || return 0
    case "$TMP_ROOT" in
        /|"")
            printf '%s\n' "error: refusing to clean unsafe temp path: $TMP_ROOT" >&2
            return 1
            ;;
        *)
            [ -d "$TMP_ROOT" ] && rm -rf "$TMP_ROOT"
            ;;
    esac
}

create_tmp_root() {
    tmp_parent="${TMPDIR:-/tmp}"
    TMP_ROOT="$(mktemp -d "${tmp_parent%/}/puread-verify-local.XXXXXX")" || {
        printf '%s\n' "error: failed to create verify-local temp directory under $tmp_parent" >&2
        exit 1
    }
}

TMP_ROOT=""
create_tmp_root
trap cleanup_tmp_root EXIT HUP INT TERM

FAILURES_FILE="$TMP_ROOT/failures.txt"
FORBIDDEN_MATCHES="$TMP_ROOT/forbidden-matches.txt"
FORBIDDEN_RG_ERRORS="$TMP_ROOT/forbidden-rg-errors.txt"
FORBIDDEN_EVIDENCE="$TASK_PREFIX-forbidden-scan.txt"
RUST_FILES="$TMP_ROOT/rust-files.txt"
LOC_ROWS="$TMP_ROOT/loc-rows.txt"
LOC_OVER="$TMP_ROOT/loc-over.txt"
SHELL_FILES="$TMP_ROOT/shell-files.txt"

: >"$FAILURES_FILE"
: >"$LOC_ROWS"
: >"$LOC_OVER"

mkdir -p "$EVIDENCE_DIR"

require_path "scripts/verify-local.sh"
require_path "scripts/package-module.sh"
require_path "rules/common"
require_path "rules/apps"
require_path "rules/sqlite"
require_path "rules/appops"
require_path "rules/components"
require_path "rules/rom"

run_capture "verify-local-sh-parse" "$TASK_PREFIX-verify-script-parse.txt" sh -n "scripts/verify-local.sh"
run_capture "cargo-fmt" "$TASK_PREFIX-cargo-fmt.txt" cargo fmt --all -- --check
run_capture "cargo-clippy" "$TASK_PREFIX-cargo-clippy.txt" cargo clippy --workspace --all-targets --all-features -- -D warnings
run_capture "cargo-test" "$TASK_PREFIX-cargo-test.txt" cargo test --workspace
run_capture "rules-validate" "$TASK_PREFIX-rules-validate.txt" cargo run -p puread-cli -- rules validate rules/common rules/apps rules/sqlite rules/appops rules/components rules/rom
run_capture "service-lifecycle" "$TASK_PREFIX-service-lifecycle.txt" scripts/test-service-lifecycle.sh
run_capture "uninstall-lifecycle" "$TASK_PREFIX-uninstall-lifecycle.txt" scripts/test-uninstall-lifecycle.sh
run_capture "package-module" "$TASK_PREFIX-package.txt" scripts/package-module.sh

write_zip_check
write_forbidden_scan
write_loc_check
write_shell_parse_check

if [ -s "$FAILURES_FILE" ]; then
    printf '%s\n' "error: local verification failed"
    sed 's/^/  - /' "$FAILURES_FILE"
    exit 1
fi

info "local verification passed"
