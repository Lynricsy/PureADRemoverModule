#!/bin/sh
set -eu

FORBIDDEN_PATTERN='hosts|DNS|dns|iptables|Clash|clash|AdGuardHome|adguardhome|mount_hosts|ad_reward|private_dns|mihomo|proxy|ProxyConfig|domain'
ALLOWED_FORBIDDEN_GUARD_PATTERN='^crates/puread-rules/src/(parse|category)\.rs:[0-9]+:[[:space:]]+"(hosts|host|dns|private_dns|domain|domains|proxy|clash|mihomo|adguardhome|iptables|iptables_network|mount_hosts|ad_reward|ad_reward_domain|ifw_clear|zygisk|root_hide)",[[:space:]]*$'

info() {
    printf '%s\n' "info: $*"
}

skip() {
    printf '%s\n' "skip: $*"
}

run() {
    info "run: $*"
    "$@"
}

scan_forbidden_path() {
    path=$1

    if [ ! -e "$path" ]; then
        return 0
    fi

    scanned=1
    status=0
    matches=$(rg -n "$FORBIDDEN_PATTERN" "$path") || status=$?
    if [ "$status" -eq 0 ]; then
        filtered_matches=$(printf '%s\n' "$matches" | filter_forbidden_matches)
        if [ "$filtered_matches" != "" ]; then
            printf '%s\n' "$filtered_matches"
            matched=1
        fi
    else
        if [ "$status" -ne 1 ]; then
            exit "$status"
        fi
    fi
}

filter_forbidden_matches() {
    while IFS= read -r match; do
        if [ "$match" = "" ]; then
            continue
        fi
        if is_allowed_forbidden_guard_match "$match"; then
            continue
        fi
        printf '%s\n' "$match"
    done
}

is_allowed_forbidden_guard_match() {
    printf '%s\n' "$1" | grep -Eq "$ALLOWED_FORBIDDEN_GUARD_PATTERN"
}

if [ "${1-}" != "" ]; then
    printf '%s\n' "error: unknown argument: $1" >&2
    exit 2
fi

if [ ! -f "AGENTS.md" ]; then
    printf '%s\n' "error: AGENTS.md is required at repository root" >&2
    exit 1
fi

run test -d ".omo/evidence"
run test -f "scripts/verify-local.sh"
run sh -n "scripts/verify-local.sh"

if [ -f "Cargo.toml" ]; then
    if command -v cargo >/dev/null 2>&1; then
        run cargo fmt --check
        run cargo check --workspace --locked
        run cargo test --workspace --locked
    else
        skip "cargo not found; Rust checks not run"
    fi
else
    skip "Cargo.toml not found; Rust checks not run"
fi

if command -v rg >/dev/null 2>&1; then
    scanned=0
    matched=0
    for path in crates/*/src src rules module modules install.sh service.sh post-fs-data.sh customize.sh; do
        scan_forbidden_path "$path"
    done
    if [ "$scanned" -eq 0 ]; then
        skip "production paths not found; forbidden-token scan not run"
    elif [ "$matched" -ne 0 ]; then
        printf '%s\n' "error: forbidden tokens found in production paths" >&2
        exit 1
    else
        info "forbidden-token scan found no production-path matches"
    fi
else
    skip "rg not found; forbidden-token scan not run"
fi

info "local verification skeleton finished; review skip lines for gates not run"
