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
PUREAD_LEDGER_PATH="$(puread_ledger_path)"
export PUREAD_RUNTIME PUREAD_ABI PUREAD_CLI_BIN PUREAD_LEDGER_PATH

puread_log "uninstall: runtime=$PUREAD_RUNTIME abi=$PUREAD_ABI cli=$PUREAD_CLI_BIN ledger=$PUREAD_LEDGER_PATH"

if [ -f "$(puread_pid_path)" ]; then
    puread_log "uninstall: daemon pid file retained for future safe-stop implementation: $(puread_pid_path)"
fi

if [ ! -f "$PUREAD_LEDGER_PATH" ]; then
    puread_write_status "uninstall_no_ledger" "no ledger found;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    puread_log "uninstall: no ledger found, nothing to plan"
    exit 0
fi

if [ ! -x "$PUREAD_CLI_BIN" ]; then
    puread_write_status "uninstall_missing_cli" "restore dry-run unavailable;cli=$PUREAD_CLI_BIN;ledger=$PUREAD_LEDGER_PATH"
    puread_log "uninstall: CLI missing, restore dry-run unavailable"
    exit 0
fi

"$PUREAD_CLI_BIN" restore --dry-run --ledger "$PUREAD_LEDGER_PATH" >>"$(puread_log_path)" 2>&1
PUREAD_RESTORE_RC="$?"

if [ "$PUREAD_RESTORE_RC" -eq 0 ]; then
    puread_write_status "uninstall_restore_planned" "restore=dry-run;ledger=$PUREAD_LEDGER_PATH;runtime=$PUREAD_RUNTIME;abi=$PUREAD_ABI"
    puread_log "uninstall: restore dry-run completed"
else
    puread_write_status "uninstall_restore_plan_failed" "rc=$PUREAD_RESTORE_RC;ledger=$PUREAD_LEDGER_PATH"
    puread_log "uninstall: restore dry-run failed rc=$PUREAD_RESTORE_RC"
fi

exit "$PUREAD_RESTORE_RC"
