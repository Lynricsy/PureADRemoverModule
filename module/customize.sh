#!/system/bin/sh

MODDIR="${0%/*}"
if [ -n "${MODPATH:-}" ]; then
    MODDIR="$MODPATH"
fi

PUREAD_MODULE_DIR="$MODDIR"
export PUREAD_MODULE_DIR
PUREAD_LIB="$MODDIR/scripts/puread-module-lib.sh"

if [ ! -r "$PUREAD_LIB" ]; then
    if command -v ui_print >/dev/null 2>&1; then
        ui_print "PureAD: missing helper script: $PUREAD_LIB"
    else
        printf '%s\n' "PureAD: missing helper script: $PUREAD_LIB"
    fi
    return 1 2>/dev/null || exit 1
fi

. "$PUREAD_LIB"

puread_init_context "$MODDIR"
puread_prepare_runtime_dirs
puread_apply_module_permissions

puread_print "- PureAD module template"
puread_print "- Runtime: $(puread_runtime_name)"
puread_print "- ABI: $(puread_select_abi)"
puread_print "- State: $(puread_status_path)"
puread_print "- Log: $(puread_log_path)"

PUREAD_DAEMON_BIN="$(puread_binary_path puread-daemon)"
PUREAD_CLI_BIN="$(puread_binary_path puread-cli)"
export PUREAD_DAEMON_BIN PUREAD_CLI_BIN

if [ -x "$PUREAD_DAEMON_BIN" ] && [ -x "$PUREAD_CLI_BIN" ]; then
    puread_write_status "installed" "runtime=$(puread_runtime_name);abi=$(puread_select_abi);binary=present"
    puread_log "customize: runtime=$(puread_runtime_name) abi=$(puread_select_abi) binaries=present"
else
    puread_write_status "missing_binary" "runtime=$(puread_runtime_name);abi=$(puread_select_abi);daemon=$PUREAD_DAEMON_BIN;cli=$PUREAD_CLI_BIN"
    puread_log "customize: runtime=$(puread_runtime_name) abi=$(puread_select_abi) binaries missing daemon=$PUREAD_DAEMON_BIN cli=$PUREAD_CLI_BIN"
    puread_print "- Native binaries are missing for this ABI."
    puread_print "- Expected daemon: $PUREAD_DAEMON_BIN"
    puread_print "- Expected CLI: $PUREAD_CLI_BIN"
fi
