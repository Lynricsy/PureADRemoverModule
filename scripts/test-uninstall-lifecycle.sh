#!/bin/sh
set -eu

die() {
    printf '%s\n' "error: $*" >&2
    exit 1
}

mk_fixture() {
    root="$1"
    mkdir -p "$root/bin/x86_64" "$root/scripts" "$root/rules" "$root/state" "$root/run" "$root/logs"
    cp module/module.prop "$root/module.prop"
    cp module/service.sh "$root/service.sh"
    cp module/uninstall.sh "$root/uninstall.sh"
    cp module/action.sh "$root/action.sh"
    cp module/customize.sh "$root/customize.sh"
    cp module/scripts/puread-module-lib.sh "$root/scripts/puread-module-lib.sh"
    cp module/scripts/puread-action-lib.sh "$root/scripts/puread-action-lib.sh"
    chmod 755 "$root/service.sh" "$root/uninstall.sh" "$root/action.sh" "$root/customize.sh" "$root/scripts/"*.sh
}

write_cli() {
    path="$1"
    log="$2"
    cat >"$path" <<SH
#!/bin/sh
printf 'cli:%s\n' "\$*" >>"$log"
case "\${1:-}" in
    restore|profile-restore)
        exit 0
        ;;
esac
SH
    chmod 755 "$path"
}

write_daemon() {
    path="$1"
    mode="$2"
    log="$3"
    cat >"$path" <<SH
#!/usr/bin/env bash
printf 'daemon:%s\n' "\$*" >>"$log"
if [ "$mode" = "ignore-term" ]; then
    exec -a "\$0" bash -c 'trap "" TERM; while :; do sleep 1; done' "\$0" "\$@"
fi
exec -a "\$0" bash -c 'trap "exit 0" TERM; while :; do sleep 1; done' "\$0" "\$@"
SH
    chmod 755 "$path"
}

start_daemon() {
    module_root="$1"
    "$module_root/bin/x86_64/puread-daemon" \
        --apply \
        --root / \
        --rules "$module_root/rules" \
        --state-dir "$module_root/state" \
        --ledger "$module_root/state/actions.jsonl" \
        --log-file "$module_root/logs/puread.log" &
    printf '%s\n' "$!" >"$module_root/run/puread-daemon.pid"
}

run_uninstall() {
    module_root="$1"
    PUREAD_FORCE_ANDROID=1 \
    ARCH=x86_64 \
    PUREAD_MODULE_STATE_DIR="$module_root/state" \
    PUREAD_MODULE_RUN_DIR="$module_root/run" \
    PUREAD_MODULE_LOG_DIR="$module_root/logs" \
    sh "$module_root/uninstall.sh" >/dev/null
}

tmp_parent="${TMPDIR:-/tmp}"
work="$(mktemp -d "${tmp_parent%/}/puread-uninstall-lifecycle.XXXXXX")"
trap 'rm -rf "$work"' EXIT HUP INT TERM

module_foreign="$work/module-foreign"
mk_fixture "$module_foreign"
write_cli "$module_foreign/bin/x86_64/puread-cli" "$work/foreign-cli.log"
sleep 30 &
foreign_pid="$!"
printf '%s\n' "$foreign_pid" >"$module_foreign/run/puread-daemon.pid"
run_uninstall "$module_foreign"
kill -0 "$foreign_pid" 2>/dev/null || die "foreign pid was stopped"
test ! -f "$module_foreign/run/puread-daemon.pid" || die "foreign pid file was not removed"
kill "$foreign_pid" 2>/dev/null || true

module_stop="$work/module-stop"
mk_fixture "$module_stop"
write_cli "$module_stop/bin/x86_64/puread-cli" "$work/stop-cli.log"
write_daemon "$module_stop/bin/x86_64/puread-daemon" stop "$work/stop-daemon.log"
start_daemon "$module_stop"
daemon_pid="$(cat "$module_stop/run/puread-daemon.pid")"
run_uninstall "$module_stop"
if kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    die "module daemon was not stopped"
fi
test ! -f "$module_stop/run/puread-daemon.pid" || die "stopped daemon pid file was not removed"

module_timeout="$work/module-timeout"
mk_fixture "$module_timeout"
write_cli "$module_timeout/bin/x86_64/puread-cli" "$work/timeout-cli.log"
write_daemon "$module_timeout/bin/x86_64/puread-daemon" ignore-term "$work/timeout-daemon.log"
start_daemon "$module_timeout"
timeout_pid="$(cat "$module_timeout/run/puread-daemon.pid")"
if run_uninstall "$module_timeout"; then
    kill -KILL "$timeout_pid" 2>/dev/null || true
    die "uninstall succeeded even though daemon ignored TERM"
fi
kill -0 "$timeout_pid" 2>/dev/null || die "timeout daemon was unexpectedly gone"
test -f "$module_timeout/run/puread-daemon.pid" || die "timeout pid file should be retained"
grep -q '^status=uninstall_daemon_stop_failed' "$module_timeout/state/status.env" || die "timeout status did not expose daemon stop failure"
grep -q '^description=PureAD status: uninstall needs attention (x86_64)' "$module_timeout/module.prop" || die "module description did not expose uninstall attention status"
kill -KILL "$timeout_pid" 2>/dev/null || true

printf '%s\n' "uninstall_lifecycle=pass"
