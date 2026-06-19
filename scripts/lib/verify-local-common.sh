usage() {
    printf '%s\n' "usage: scripts/verify-local.sh"
}

info() {
    printf '%s\n' "info: $*"
}

fail() {
    printf '%s\n' "error: $*" >&2
    printf '%s\n' "$*" >>"$FAILURES_FILE"
}

command_line() {
    printf '%s' "$1"
    shift
    for arg in "$@"; do
        printf ' %s' "$arg"
    done
    printf '\n'
}

run_capture() {
    label="$1"
    evidence="$2"
    shift 2

    info "run: $(command_line "$@")"
    {
        printf 'gate=%s\n' "$label"
        printf 'command='
        command_line "$@"
    } >"$evidence"

    if "$@" >>"$evidence" 2>&1; then
        printf 'result=pass\n' >>"$evidence"
        info "pass: $label"
    else
        rc=$?
        printf 'result=fail\n' >>"$evidence"
        printf 'exit_code=%s\n' "$rc" >>"$evidence"
        fail "$label failed; see $evidence"
    fi
}

require_path() {
    if [ ! -e "$1" ]; then
        fail "missing required path: $1"
    fi
}
