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

PUREAD_ACTION="${1:-status}"
PUREAD_CLI_BIN="$(puread_binary_path puread-cli)"
export PUREAD_ACTION PUREAD_CLI_BIN

case "$PUREAD_ACTION" in
    status)
        puread_action_status
        ;;
    diagnose|diagnostic|diagnostics)
        puread_action_diagnostics
        ;;
    scan-dry-run)
        puread_action_scan_dry_run
        ;;
    restore-dry-run)
        puread_action_restore_dry_run
        ;;
    *)
        puread_print "PureAD actions: status | diagnose | scan-dry-run | restore-dry-run"
        puread_print "Unknown action: $PUREAD_ACTION"
        exit 2
        ;;
esac
