#!/system/bin/sh

puread_action_status() {
    puread_print "PureAD status"
    puread_print "runtime=$(puread_runtime_name)"
    puread_print "abi=$(puread_select_abi)"
    puread_print "module_dir=$PUREAD_MODDIR"
    puread_print "state=$(puread_status_path)"
    puread_print "log=$(puread_log_path)"
    puread_print "daemon=$(puread_binary_path puread-daemon)"
    puread_print "cli=$(puread_binary_path puread-cli)"

    if [ -f "$(puread_status_path)" ]; then
        puread_print "--- status file ---"
        sed -n '1,80p' "$(puread_status_path)" 2>/dev/null || true
    else
        puread_print "status_file=missing"
    fi

    if [ ! -x "$(puread_binary_path puread-cli)" ] || [ ! -x "$(puread_binary_path puread-daemon)" ]; then
        puread_print "native_binary=missing_until_T25"
    fi
}

puread_action_diagnostics() {
    puread_action_status
    puread_print "--- diagnostics ---"
    puread_print "android=$(puread_is_android && printf '%s' yes || printf '%s' no)"
    puread_print "ledger=$(puread_ledger_path)"
    puread_print "pid=$(puread_pid_path)"
    puread_print "lock=$(puread_lock_path)"
}

puread_action_scan_dry_run() {
    PUREAD_CLI="$(puread_binary_path puread-cli)"
    if [ ! -x "$PUREAD_CLI" ]; then
        puread_print "PureAD CLI missing: $PUREAD_CLI"
        return 1
    fi

    PUREAD_RULES_DIR="${PUREAD_RULES_DIR:-$PUREAD_MODDIR/rules}"
    PUREAD_SCAN_ROOT="${PUREAD_SCAN_ROOT:-/data}"
    "$PUREAD_CLI" scan --dry-run --rules "$PUREAD_RULES_DIR" --root "$PUREAD_SCAN_ROOT"
}

puread_action_restore_dry_run() {
    PUREAD_CLI="$(puread_binary_path puread-cli)"
    if [ ! -x "$PUREAD_CLI" ]; then
        puread_print "PureAD CLI missing: $PUREAD_CLI"
        return 1
    fi

    "$PUREAD_CLI" restore --dry-run --ledger "$(puread_ledger_path)"
}
