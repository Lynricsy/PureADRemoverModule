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

if ! puread_is_android && [ -n "${PUREAD_MODULE_STATE_DIR:-}" ] && [ -z "${PUREAD_MODULE_RUN_DIR:-}" ]; then
    PUREAD_MODULE_RUN_DIR="$PUREAD_MODULE_STATE_DIR/run"
    export PUREAD_MODULE_RUN_DIR
fi

PUREAD_RUNTIME="$(puread_runtime_name)"
PUREAD_ABI="$(puread_select_abi)"
PUREAD_DAEMON_BIN="$(puread_binary_path puread-daemon)"
PUREAD_LOG_PATH="$(puread_log_path)"
PUREAD_PID_PATH="$(puread_pid_path)"
export PUREAD_RUNTIME PUREAD_ABI PUREAD_DAEMON_BIN PUREAD_LOG_PATH PUREAD_PID_PATH

if ! puread_is_android; then
    puread_print "PureAD service host dry-run"
    puread_print "status=host_dry_run"
    puread_print "runtime=$PUREAD_RUNTIME"
    puread_print "abi=$PUREAD_ABI"
    puread_print "daemon=$PUREAD_DAEMON_BIN"
    puread_print "state=$(puread_status_path)"
    puread_print "log=$(puread_log_path)"

    if [ -n "${PUREAD_MODULE_STATE_DIR:-}" ]; then
        puread_write_status "host_dry_run" "service skipped outside Android;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    fi
    if [ -n "${PUREAD_MODULE_LOG_DIR:-}" ]; then
        puread_log "service: host dry-run, Android-only daemon launch skipped"
    fi
    exit 0
fi

puread_prepare_runtime_dirs
puread_apply_module_permissions
puread_log "service: runtime=$PUREAD_RUNTIME abi=$PUREAD_ABI daemon=$PUREAD_DAEMON_BIN"

if [ "${PUREAD_DAEMON_DISABLE:-0}" = "1" ]; then
    puread_write_status "daemon_disabled" "PUREAD_DAEMON_DISABLE=1;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    puread_log "service: daemon disabled by environment"
    exit 0
fi

if [ ! -x "$PUREAD_DAEMON_BIN" ]; then
    puread_write_status "missing_binary" "daemon=$PUREAD_DAEMON_BIN;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    puread_log "service: daemon binary missing or not executable: $PUREAD_DAEMON_BIN"
    exit 0
fi

if puread_pid_running "$PUREAD_PID_PATH"; then
    PUREAD_OLD_PID="$(cat "$PUREAD_PID_PATH" 2>/dev/null || printf '%s' unknown)"
    puread_write_status "daemon_already_running" "pid=$PUREAD_OLD_PID;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    puread_log "service: daemon already running pid=$PUREAD_OLD_PID"
    exit 0
fi

PUREAD_DRY_RUN="${PUREAD_DRY_RUN:-1}"
export PUREAD_DRY_RUN

if [ "$PUREAD_DRY_RUN" = "0" ]; then
    "$PUREAD_DAEMON_BIN" >>"$PUREAD_LOG_PATH" 2>&1 &
else
    "$PUREAD_DAEMON_BIN" --dry-run --state-dir "$(puread_state_dir)" --log-file "$PUREAD_LOG_PATH" >>"$PUREAD_LOG_PATH" 2>&1 &
fi

PUREAD_DAEMON_PID="$!"
printf '%s\n' "$PUREAD_DAEMON_PID" >"$PUREAD_PID_PATH"
chmod 600 "$PUREAD_PID_PATH" 2>/dev/null || true

puread_write_status "daemon_started" "pid=$PUREAD_DAEMON_PID;dry_run=$PUREAD_DRY_RUN;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
puread_log "service: daemon started pid=$PUREAD_DAEMON_PID dry_run=$PUREAD_DRY_RUN"
