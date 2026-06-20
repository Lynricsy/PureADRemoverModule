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
PUREAD_CLI_BIN="$(puread_binary_path puread-cli)"
PUREAD_LOG_PATH="$(puread_log_path)"
PUREAD_PID_PATH="$(puread_pid_path)"
export PUREAD_RUNTIME PUREAD_ABI PUREAD_DAEMON_BIN PUREAD_CLI_BIN PUREAD_LOG_PATH PUREAD_PID_PATH

if ! puread_is_android; then
    puread_print "PureAD service host dry-run"
    puread_print "status=host_dry_run"
    puread_print "runtime=$PUREAD_RUNTIME"
    puread_print "abi=$PUREAD_ABI"
    puread_print "daemon=$PUREAD_DAEMON_BIN"
    puread_print "cli=$PUREAD_CLI_BIN"
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
puread_log "service: runtime=$PUREAD_RUNTIME abi=$PUREAD_ABI daemon=$PUREAD_DAEMON_BIN cli=$PUREAD_CLI_BIN"

PUREAD_AUTO_APPLY_FAILURES=0

puread_auto_apply_once() {
    PUREAD_AUTO_APPLY_FAILURES=0
    if [ "${PUREAD_AUTO_APPLY:-1}" != "1" ]; then
        puread_log "service: auto apply disabled by PUREAD_AUTO_APPLY=$PUREAD_AUTO_APPLY"
        return 0
    fi
    if [ ! -x "$PUREAD_CLI_BIN" ]; then
        puread_log "service: auto apply skipped, CLI missing: $PUREAD_CLI_BIN"
        return 0
    fi

    PUREAD_MARKER="$(puread_auto_apply_marker)"
    if [ -f "$PUREAD_MARKER" ] && [ "${PUREAD_AUTO_APPLY_FORCE:-0}" != "1" ]; then
        puread_log "service: auto apply already completed for this module version"
        return 0
    fi

    PUREAD_RULES_DIR="${PUREAD_RULES_DIR:-$MODDIR/rules}"
    PUREAD_PROFILE_ROOT="${PUREAD_PROFILE_ROOT:-/}"
    PUREAD_AUTO_PROFILES="${PUREAD_AUTO_PROFILES:-conservative sdk_cache sqlite}"
    PUREAD_AUTO_OUTPUT="$(puread_state_dir)/auto-apply-current.json"
    PUREAD_AUTO_SUMMARY="$(puread_state_dir)/auto-apply-summary.log"
    PUREAD_AUTO_FAILURES=0
    : >"$PUREAD_AUTO_SUMMARY" 2>/dev/null || true
    chmod 600 "$PUREAD_AUTO_SUMMARY" 2>/dev/null || true

    for PUREAD_PROFILE_NAME in $PUREAD_AUTO_PROFILES; do
        puread_log "service: auto apply profile=$PUREAD_PROFILE_NAME start"
        if "$PUREAD_CLI_BIN" apply-profile "$PUREAD_PROFILE_NAME" --execute --rules "$PUREAD_RULES_DIR" --root "$PUREAD_PROFILE_ROOT" --module-root "$MODDIR" >"$PUREAD_AUTO_OUTPUT" 2>>"$PUREAD_LOG_PATH"; then
            PUREAD_PROFILE_RC=0
        else
            PUREAD_PROFILE_RC="$?"
        fi
        chmod 600 "$PUREAD_AUTO_OUTPUT" 2>/dev/null || true
        if [ "$PUREAD_PROFILE_RC" -eq 0 ] && "$PUREAD_CLI_BIN" json-field-is-zero --file "$PUREAD_AUTO_OUTPUT" --field failed 2>>"$PUREAD_LOG_PATH"; then
            printf 'profile=%s status=done\n' "$PUREAD_PROFILE_NAME" >>"$PUREAD_AUTO_SUMMARY" 2>/dev/null || true
            puread_log "service: auto apply profile=$PUREAD_PROFILE_NAME done"
        else
            PUREAD_AUTO_FAILURES=$((PUREAD_AUTO_FAILURES + 1))
            printf 'profile=%s status=failed rc=%s\n' "$PUREAD_PROFILE_NAME" "$PUREAD_PROFILE_RC" >>"$PUREAD_AUTO_SUMMARY" 2>/dev/null || true
            puread_log "service: auto apply profile=$PUREAD_PROFILE_NAME failed rc=$PUREAD_PROFILE_RC"
        fi
    done

    PUREAD_AUTO_APPLY_FAILURES="$PUREAD_AUTO_FAILURES"
    if [ "$PUREAD_AUTO_FAILURES" -eq 0 ]; then
        {
            printf 'version=%s\n' "$(puread_module_version)"
            printf 'profiles=%s\n' "$PUREAD_AUTO_PROFILES"
            printf 'failures=0\n'
            printf 'updated_at=%s\n' "$(puread_now)"
        } >"$PUREAD_MARKER" 2>/dev/null || true
        chmod 600 "$PUREAD_MARKER" 2>/dev/null || true
        puread_write_status "auto_apply_complete" "profiles=$PUREAD_AUTO_PROFILES;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    else
        rm -f "$PUREAD_MARKER" 2>/dev/null || true
        puread_write_status "auto_apply_partial" "failures=$PUREAD_AUTO_FAILURES;profiles=$PUREAD_AUTO_PROFILES"
    fi
}

puread_auto_apply_once
case "${PUREAD_AUTO_APPLY_FAILURES:-0}" in
    ''|*[!0-9]*)
        PUREAD_AUTO_APPLY_FAILURES=0
        ;;
esac

if [ "${PUREAD_DAEMON_DISABLE:-0}" = "1" ]; then
    if [ "$PUREAD_AUTO_APPLY_FAILURES" -gt 0 ]; then
        puread_write_status "daemon_disabled_with_profile_errors" "PUREAD_DAEMON_DISABLE=1;auto_failures=$PUREAD_AUTO_APPLY_FAILURES;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    else
        puread_write_status "daemon_disabled" "PUREAD_DAEMON_DISABLE=1;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    fi
    puread_log "service: daemon disabled by environment after auto apply"
    exit 0
fi

if [ ! -x "$PUREAD_DAEMON_BIN" ]; then
    puread_write_status "missing_binary" "daemon=$PUREAD_DAEMON_BIN;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    puread_log "service: daemon binary missing or not executable: $PUREAD_DAEMON_BIN"
    exit 0
fi

if puread_pid_matches_daemon "$PUREAD_PID_PATH"; then
    PUREAD_OLD_PID="$(cat "$PUREAD_PID_PATH" 2>/dev/null || printf '%s' unknown)"
    if [ "$PUREAD_AUTO_APPLY_FAILURES" -gt 0 ]; then
        puread_write_status "daemon_started_with_profile_errors" "pid=$PUREAD_OLD_PID;auto_failures=$PUREAD_AUTO_APPLY_FAILURES;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    else
        puread_write_status "daemon_already_running" "pid=$PUREAD_OLD_PID;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    fi
    puread_log "service: daemon already running pid=$PUREAD_OLD_PID"
    exit 0
elif [ -f "$PUREAD_PID_PATH" ]; then
    rm -f "$PUREAD_PID_PATH" 2>/dev/null || true
    puread_log "service: removed stale or foreign daemon pid file"
fi

PUREAD_DRY_RUN="${PUREAD_DRY_RUN:-0}"
export PUREAD_DRY_RUN

if [ "$PUREAD_DRY_RUN" = "0" ]; then
    "$PUREAD_DAEMON_BIN" --apply --root / --rules "$MODDIR/rules" --state-dir "$(puread_state_dir)" --ledger "$(puread_ledger_path)" --log-file "$PUREAD_LOG_PATH" >>"$PUREAD_LOG_PATH" 2>&1 &
else
    "$PUREAD_DAEMON_BIN" --dry-run --root / --rules "$MODDIR/rules" --state-dir "$(puread_state_dir)" --log-file "$PUREAD_LOG_PATH" >>"$PUREAD_LOG_PATH" 2>&1 &
fi

PUREAD_DAEMON_PID="$!"
printf '%s\n' "$PUREAD_DAEMON_PID" >"$PUREAD_PID_PATH"
chmod 600 "$PUREAD_PID_PATH" 2>/dev/null || true

if [ "$PUREAD_AUTO_APPLY_FAILURES" -gt 0 ]; then
    puread_write_status "daemon_started_with_profile_errors" "pid=$PUREAD_DAEMON_PID;auto_failures=$PUREAD_AUTO_APPLY_FAILURES;dry_run=$PUREAD_DRY_RUN;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
else
    puread_write_status "daemon_started" "pid=$PUREAD_DAEMON_PID;dry_run=$PUREAD_DRY_RUN;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
fi
puread_log "service: daemon started pid=$PUREAD_DAEMON_PID dry_run=$PUREAD_DRY_RUN"
