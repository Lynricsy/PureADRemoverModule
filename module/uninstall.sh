#!/system/bin/sh

MODDIR="${0%/*}"
PUREAD_MODULE_DIR="$MODDIR"
export PUREAD_MODULE_DIR
PUREAD_LIB="$MODDIR/scripts/puread-module-lib.sh"

if [ ! -r "$PUREAD_LIB" ]; then
    printf '%s\n' "PureAD: missing helper script: $PUREAD_LIB"
    exit 1
fi

. "$PUREAD_LIB"

puread_init_context "$MODDIR"
puread_prepare_runtime_dirs

PUREAD_RUNTIME="$(puread_runtime_name)"
PUREAD_ABI="$(puread_select_abi)"
PUREAD_CLI_BIN="$(puread_binary_path puread-cli)"
PUREAD_DAEMON_BIN="$(puread_binary_path puread-daemon)"
PUREAD_LEDGER_PATH="$(puread_ledger_path)"
PUREAD_PROFILE_LEDGER_PATH="$(puread_profile_ledger_path)"
export PUREAD_RUNTIME PUREAD_ABI PUREAD_CLI_BIN PUREAD_DAEMON_BIN PUREAD_LEDGER_PATH PUREAD_PROFILE_LEDGER_PATH

puread_log "uninstall: runtime=$PUREAD_RUNTIME abi=$PUREAD_ABI cli=$PUREAD_CLI_BIN daemon=$PUREAD_DAEMON_BIN ledger=$PUREAD_LEDGER_PATH"

puread_wait_daemon_exit() {
    PUREAD_WAIT_PID="$1"
    PUREAD_WAIT_LEFT=5
    while [ "$PUREAD_WAIT_LEFT" -gt 0 ]; do
        if ! kill -0 "$PUREAD_WAIT_PID" 2>/dev/null; then
            return 0
        fi
        sleep 1
        PUREAD_WAIT_LEFT=$((PUREAD_WAIT_LEFT - 1))
    done
    ! kill -0 "$PUREAD_WAIT_PID" 2>/dev/null
}

PUREAD_DAEMON_STOP_RC=0

if puread_pid_matches_daemon "$(puread_pid_path)"; then
    PUREAD_STOP_PID="$(cat "$(puread_pid_path)" 2>/dev/null || printf '%s' "")"
    kill -TERM "$PUREAD_STOP_PID" 2>/dev/null || true
    if puread_wait_daemon_exit "$PUREAD_STOP_PID"; then
        rm -f "$(puread_pid_path)" 2>/dev/null || true
        puread_log "uninstall: daemon stopped pid=$PUREAD_STOP_PID"
    else
        PUREAD_DAEMON_STOP_RC=1
        puread_log "uninstall: daemon stop timed out pid=$PUREAD_STOP_PID"
    fi
elif [ -f "$(puread_pid_path)" ]; then
    rm -f "$(puread_pid_path)" 2>/dev/null || true
    puread_log "uninstall: removed stale daemon pid file"
fi

if [ ! -x "$PUREAD_CLI_BIN" ]; then
    if [ "$PUREAD_DAEMON_STOP_RC" -ne 0 ]; then
        puread_write_status "uninstall_daemon_stop_failed" "restore=dry-run-unavailable;daemon_stop_rc=$PUREAD_DAEMON_STOP_RC;cli=$PUREAD_CLI_BIN;ledger=$PUREAD_LEDGER_PATH"
        puread_log "uninstall: CLI missing after daemon stop failure"
        exit "$PUREAD_DAEMON_STOP_RC"
    fi
    puread_write_status "uninstall_missing_cli" "restore dry-run unavailable;cli=$PUREAD_CLI_BIN;ledger=$PUREAD_LEDGER_PATH"
    puread_log "uninstall: CLI missing, restore dry-run unavailable"
    exit 0
fi

PUREAD_RESTORE_RC="$PUREAD_DAEMON_STOP_RC"

if [ -f "$PUREAD_LEDGER_PATH" ]; then
    if "$PUREAD_CLI_BIN" restore --dry-run --ledger "$PUREAD_LEDGER_PATH" >>"$(puread_log_path)" 2>&1; then
        PUREAD_RESTORE_RC=0
    else
        PUREAD_RESTORE_RC="$?"
    fi
else
    puread_log "uninstall: no file ledger found"
fi

if [ -f "$PUREAD_PROFILE_LEDGER_PATH" ]; then
    if "$PUREAD_CLI_BIN" profile-restore --dry-run --module-root "$MODDIR" >>"$(puread_log_path)" 2>&1; then
        PUREAD_PROFILE_RESTORE_RC=0
    else
        PUREAD_PROFILE_RESTORE_RC="$?"
    fi
    if [ "$PUREAD_PROFILE_RESTORE_RC" -ne 0 ]; then
        PUREAD_RESTORE_RC="$PUREAD_PROFILE_RESTORE_RC"
    fi
else
    puread_log "uninstall: no profile ledger found"
fi

if [ "$PUREAD_RESTORE_RC" -eq 0 ]; then
    puread_write_status "uninstall_restore_planned" "restore=dry-run;ledger=$PUREAD_LEDGER_PATH;profile_ledger=$PUREAD_PROFILE_LEDGER_PATH;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    puread_log "uninstall: restore dry-run completed"
else
    if [ "$PUREAD_DAEMON_STOP_RC" -ne 0 ]; then
        puread_write_status "uninstall_daemon_stop_failed" "daemon_stop_rc=$PUREAD_DAEMON_STOP_RC;restore_rc=$PUREAD_RESTORE_RC;ledger=$PUREAD_LEDGER_PATH"
        puread_log "uninstall: daemon stop failed rc=$PUREAD_DAEMON_STOP_RC restore_rc=$PUREAD_RESTORE_RC"
    else
        puread_write_status "uninstall_restore_plan_failed" "rc=$PUREAD_RESTORE_RC;ledger=$PUREAD_LEDGER_PATH"
        puread_log "uninstall: restore dry-run failed rc=$PUREAD_RESTORE_RC"
    fi
fi

exit "$PUREAD_RESTORE_RC"
