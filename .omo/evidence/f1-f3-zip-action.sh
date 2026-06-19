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

puread_action_usage() {
    puread_print "Usage: sh action.sh <action> [profile]"
    puread_print "PureAD actions:"
    puread_print "  status"
    puread_print "  diagnose"
    puread_print "  scan-dry-run"
    puread_print "  restore-dry-run"
    puread_print "  apply-profile-dry-run <profile>"
    puread_print "  apply-profile-execute <profile>"
    puread_print "  profile-restore-dry-run"
    puread_print "  profile-restore-execute"
    puread_print "Environment:"
    puread_print "  PUREAD_PROFILE=<profile> may replace the apply-profile profile argument"
    puread_print "  PUREAD_RULES_DIR=<path> overrides profile rule directory"
    puread_print "  PUREAD_PROFILE_ROOT=<path> overrides profile target root"
}

puread_require_profile_arg() {
    PUREAD_REQUESTED_PROFILE=""
    if [ -n "${1:-}" ]; then
        PUREAD_REQUESTED_PROFILE="$1"
        return 0
    fi
    if [ -n "${PUREAD_PROFILE:-}" ]; then
        PUREAD_REQUESTED_PROFILE="$PUREAD_PROFILE"
        return 0
    fi

    puread_print "PureAD profile is required: pass it as the second argument or set PUREAD_PROFILE."
    return 2
}

puread_require_no_extra_arg() {
    if [ -n "${1:-}" ]; then
        puread_print "PureAD action does not accept an extra argument: $1"
        return 2
    fi
    return 0
}

puread_action_apply_profile() {
    PUREAD_APPLY_MODE="$1"
    PUREAD_PROFILE_NAME="$2"
    PUREAD_CLI="$(puread_binary_path puread-cli)"
    if [ ! -x "$PUREAD_CLI" ]; then
        puread_print "PureAD CLI missing: $PUREAD_CLI"
        return 1
    fi

    PUREAD_RULES_DIR="${PUREAD_RULES_DIR:-$PUREAD_MODDIR/rules}"
    PUREAD_PROFILE_ROOT="${PUREAD_PROFILE_ROOT:-/}"
    "$PUREAD_CLI" apply-profile "$PUREAD_PROFILE_NAME" "--$PUREAD_APPLY_MODE" --rules "$PUREAD_RULES_DIR" --root "$PUREAD_PROFILE_ROOT" --module-root "$PUREAD_MODDIR"
}

puread_action_restore_profile() {
    PUREAD_RESTORE_MODE="$1"
    PUREAD_CLI="$(puread_binary_path puread-cli)"
    if [ ! -x "$PUREAD_CLI" ]; then
        puread_print "PureAD CLI missing: $PUREAD_CLI"
        return 1
    fi

    puread_prepare_profile_ledger || return $?
    "$PUREAD_CLI" profile-restore "--$PUREAD_RESTORE_MODE" --module-root "$PUREAD_MODDIR"
}

puread_prepare_profile_ledger() {
    PUREAD_PROFILE_LEDGER_DIR="$PUREAD_MODDIR/state"
    PUREAD_PROFILE_LEDGER_FILE="$PUREAD_PROFILE_LEDGER_DIR/profile-actions.jsonl"
    if [ -L "$PUREAD_PROFILE_LEDGER_DIR" ] || [ -L "$PUREAD_PROFILE_LEDGER_FILE" ]; then
        puread_print "PureAD profile ledger path must not be a symlink."
        return 1
    fi
    if [ -e "$PUREAD_PROFILE_LEDGER_DIR" ] && [ ! -d "$PUREAD_PROFILE_LEDGER_DIR" ]; then
        puread_print "PureAD profile ledger parent is not a directory: $PUREAD_PROFILE_LEDGER_DIR"
        return 1
    fi
    if [ -e "$PUREAD_PROFILE_LEDGER_FILE" ] && [ ! -f "$PUREAD_PROFILE_LEDGER_FILE" ]; then
        puread_print "PureAD profile ledger is not a file: $PUREAD_PROFILE_LEDGER_FILE"
        return 1
    fi

    mkdir -p "$PUREAD_PROFILE_LEDGER_DIR" 2>/dev/null || return 1
    chmod 700 "$PUREAD_PROFILE_LEDGER_DIR" 2>/dev/null || true
    if [ ! -f "$PUREAD_PROFILE_LEDGER_FILE" ]; then
        : >"$PUREAD_PROFILE_LEDGER_FILE" || return 1
        chmod 600 "$PUREAD_PROFILE_LEDGER_FILE" 2>/dev/null || true
    fi
}

case "$PUREAD_ACTION" in
    -h|--help|help)
        puread_action_usage
        ;;
    status)
        puread_require_no_extra_arg "${2:-}" || exit $?
        puread_action_status
        ;;
    diagnose|diagnostic|diagnostics)
        puread_require_no_extra_arg "${2:-}" || exit $?
        puread_action_diagnostics
        ;;
    scan-dry-run)
        puread_require_no_extra_arg "${2:-}" || exit $?
        puread_action_scan_dry_run
        ;;
    restore-dry-run)
        puread_require_no_extra_arg "${2:-}" || exit $?
        puread_action_restore_dry_run
        ;;
    apply-profile-dry-run|profile-apply-dry-run)
        puread_require_profile_arg "${2:-}" || exit $?
        puread_require_no_extra_arg "${3:-}" || exit $?
        puread_action_apply_profile "dry-run" "$PUREAD_REQUESTED_PROFILE"
        ;;
    apply-profile-execute|profile-apply-execute)
        puread_require_profile_arg "${2:-}" || exit $?
        puread_require_no_extra_arg "${3:-}" || exit $?
        puread_action_apply_profile "execute" "$PUREAD_REQUESTED_PROFILE"
        ;;
    profile-restore-dry-run)
        puread_require_no_extra_arg "${2:-}" || exit $?
        puread_action_restore_profile "dry-run"
        ;;
    profile-restore-execute)
        puread_require_no_extra_arg "${2:-}" || exit $?
        puread_action_restore_profile "execute"
        ;;
    *)
        puread_action_usage
        puread_print "Unknown action: $PUREAD_ACTION"
        exit 2
        ;;
esac
