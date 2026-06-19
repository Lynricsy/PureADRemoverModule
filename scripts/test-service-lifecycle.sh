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
    fail_profile="${3:-}"
    cat >"$path" <<SH
#!/bin/sh
printf 'cli:%s\n' "\$*" >>"$log"
case "\${1:-}" in
    apply-profile)
        if [ "\${2:-}" = "$fail_profile" ]; then
            printf '{"failed":1}\n'
            exit 0
        fi
        printf '{"failed":0}\n'
        ;;
    json-field-is-zero)
        file=""
        while [ "\$#" -gt 0 ]; do
            case "\$1" in
                --file)
                    shift
                    file="\${1:-}"
                    ;;
            esac
            shift || true
        done
        grep '"failed"[[:space:]]*:[[:space:]]*0' "\$file" >/dev/null 2>&1
        ;;
    restore|profile-restore)
        exit 0
        ;;
esac
SH
    chmod 755 "$path"
}

write_daemon() {
    path="$1"
    log="$2"
    cat >"$path" <<SH
#!/bin/sh
printf 'daemon:%s\n' "\$*" >>"$log"
sleep 2
SH
    chmod 755 "$path"
}

run_service() {
    module_root="$1"
    PUREAD_FORCE_ANDROID=1 \
    ARCH=x86_64 \
    PUREAD_MODULE_STATE_DIR="$module_root/state" \
    PUREAD_MODULE_RUN_DIR="$module_root/run" \
    PUREAD_MODULE_LOG_DIR="$module_root/logs" \
    sh "$module_root/service.sh" >/dev/null
}

run_customize() {
    module_root="$1"
    PUREAD_FORCE_ANDROID=1 \
    ARCH=x86_64 \
    MODPATH="$module_root" \
    sh "$module_root/customize.sh" >/dev/null
}

tmp_parent="${TMPDIR:-/tmp}"
work="$(mktemp -d "${tmp_parent%/}/puread-service-lifecycle.XXXXXX")"
trap 'rm -rf "$work"' EXIT HUP INT TERM

module_install="$work/module-install"
mk_fixture "$module_install"
write_cli "$module_install/bin/x86_64/puread-cli" "$work/install-cli.log"
write_daemon "$module_install/bin/x86_64/puread-daemon" "$work/install-daemon.log"
run_customize "$module_install"
grep -q '^status=installed' "$module_install/state/status.env" || die "customize did not write installed status"
grep -q '^description=PureAD status: installed; reboot to activate (x86_64)' "$module_install/module.prop" || die "module description did not expose install status"

module_ok="$work/module-ok"
mk_fixture "$module_ok"
write_cli "$module_ok/bin/x86_64/puread-cli" "$work/ok-cli.log"
write_daemon "$module_ok/bin/x86_64/puread-daemon" "$work/ok-daemon.log"
run_service "$module_ok"

grep -q 'profile=conservative status=done' "$module_ok/state/auto-apply-summary.log" || die "conservative profile was not auto-applied"
grep -q 'profile=rom status=done' "$module_ok/state/auto-apply-summary.log" || die "rom profile was not auto-applied"
test -f "$module_ok/state/auto-apply-$(sed -n 's/^version=//p' "$module_ok/module.prop").done" || die "auto apply marker missing"
grep -q '^daemon:' "$work/ok-daemon.log" || die "daemon was not started"
grep -q '^description=PureAD status: active; daemon running (x86_64)' "$module_ok/module.prop" || die "module description did not expose running daemon status"

if [ -f "$module_ok/run/puread-daemon.pid" ]; then
    daemon_pid="$(cat "$module_ok/run/puread-daemon.pid")"
    kill "$daemon_pid" 2>/dev/null || true
fi

module_stale="$work/module-stale"
mk_fixture "$module_stale"
write_cli "$module_stale/bin/x86_64/puread-cli" "$work/stale-cli.log"
write_daemon "$module_stale/bin/x86_64/puread-daemon" "$work/stale-daemon.log"
sleep 30 &
foreign_pid="$!"
printf '%s\n' "$foreign_pid" >"$module_stale/run/puread-daemon.pid"
run_service "$module_stale"
kill -0 "$foreign_pid" 2>/dev/null || die "foreign pid was stopped"
grep -q '^daemon:' "$work/stale-daemon.log" || die "service did not start daemon after stale pid"
kill "$foreign_pid" 2>/dev/null || true

if [ -f "$module_stale/run/puread-daemon.pid" ]; then
    daemon_pid="$(cat "$module_stale/run/puread-daemon.pid")"
    kill "$daemon_pid" 2>/dev/null || true
fi

module_fail="$work/module-fail"
mk_fixture "$module_fail"
write_cli "$module_fail/bin/x86_64/puread-cli" "$work/fail-cli.log" sqlite
write_daemon "$module_fail/bin/x86_64/puread-daemon" "$work/fail-daemon.log"
PUREAD_DAEMON_DISABLE=1 run_service "$module_fail"
grep -q 'profile=sqlite status=failed' "$module_fail/state/auto-apply-summary.log" || die "failed profile was not recorded"
test ! -f "$module_fail/state/auto-apply-$(sed -n 's/^version=//p' "$module_fail/module.prop").done" || die "failed auto apply wrote marker"
grep -q '^description=PureAD status: profile errors; daemon disabled (x86_64)' "$module_fail/module.prop" || die "module description did not expose profile error status"

printf '%s\n' "service_lifecycle=pass"
