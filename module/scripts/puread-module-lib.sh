#!/system/bin/sh

if [ -z "${MODDIR:-}" ]; then
    MODDIR="${0%/*}"
fi

puread_init_context() {
    PUREAD_MODDIR="$1"
    PUREAD_MODULE_ID="$(sed -n 's/^id=//p' "$PUREAD_MODDIR/module.prop" 2>/dev/null | head -n 1)"
    if [ -z "$PUREAD_MODULE_ID" ]; then
        PUREAD_MODULE_ID="PureAD"
    fi
    export PUREAD_MODDIR PUREAD_MODULE_ID
}

puread_print() {
    if command -v ui_print >/dev/null 2>&1; then
        ui_print "$*"
    else
        printf '%s\n' "$*"
    fi
}

puread_now() {
    date '+%Y-%m-%dT%H:%M:%S%z' 2>/dev/null || printf '%s' "unknown-time"
}

puread_runtime_name() {
    if [ "${APATCH:-}" = "true" ] || [ "${KERNELPATCH:-}" = "true" ] || [ -n "${APATCH_VER_CODE:-}" ] || [ -d /data/adb/ap ]; then
        printf '%s' "APatch"
    elif [ "${KSU:-}" = "true" ] || [ -n "${KSU_VER_CODE:-}" ] || [ -d /data/adb/ksu ]; then
        printf '%s' "KernelSU"
    elif [ -n "${MAGISK_VER_CODE:-}" ] || [ -n "${MAGISK_VER:-}" ] || [ -d /data/adb/magisk ]; then
        printf '%s' "Magisk"
    else
        printf '%s' "unknown"
    fi
}

puread_is_android() {
    [ "${PUREAD_FORCE_ANDROID:-0}" = "1" ] && return 0
    [ -d /system ] && [ -d /data/adb ]
}

puread_getprop() {
    if command -v getprop >/dev/null 2>&1; then
        getprop "$1" 2>/dev/null
    else
        printf '%s' ""
    fi
}

puread_arch_input() {
    if [ -n "${ARCH:-}" ]; then
        printf '%s' "$ARCH"
        return
    fi

    PUREAD_PROP_ABI="$(puread_getprop ro.product.cpu.abi)"
    if [ -n "$PUREAD_PROP_ABI" ]; then
        printf '%s' "$PUREAD_PROP_ABI"
        return
    fi

    PUREAD_PROP_ABILIST="$(puread_getprop ro.product.cpu.abilist)"
    if [ -n "$PUREAD_PROP_ABILIST" ]; then
        printf '%s' "$PUREAD_PROP_ABILIST"
        return
    fi

    uname -m 2>/dev/null || printf '%s' "unknown"
}

puread_select_abi() {
    PUREAD_ARCH_RAW="$(puread_arch_input)"

    case "$PUREAD_ARCH_RAW" in
        arm64|arm64-v8a|aarch64|*arm64-v8a*)
            printf '%s' "arm64-v8a"
            ;;
        arm|armeabi-v7a|armv7l|*armeabi-v7a*)
            printf '%s' "armeabi-v7a"
            ;;
        x64|x86_64|amd64|*x86_64*)
            printf '%s' "x86_64"
            ;;
        x86|i386|i686|*x86*)
            printf '%s' "x86"
            ;;
        riscv64|*riscv64*)
            printf '%s' "riscv64"
            ;;
        *)
            printf '%s' "unknown"
            ;;
    esac
}

puread_binary_path() {
    printf '%s/bin/%s/%s' "$PUREAD_MODDIR" "$(puread_select_abi)" "$1"
}

puread_state_dir() {
    printf '%s' "${PUREAD_MODULE_STATE_DIR:-$PUREAD_MODDIR/state}"
}

puread_run_dir() {
    printf '%s' "${PUREAD_MODULE_RUN_DIR:-$PUREAD_MODDIR/run}"
}

puread_log_dir() {
    printf '%s' "${PUREAD_MODULE_LOG_DIR:-$PUREAD_MODDIR/logs}"
}

puread_status_path() {
    printf '%s/status.env' "$(puread_state_dir)"
}

puread_log_path() {
    printf '%s/puread.log' "$(puread_log_dir)"
}

puread_pid_path() {
    printf '%s/puread-daemon.pid' "$(puread_run_dir)"
}

puread_lock_path() {
    printf '%s/puread.lock' "$(puread_run_dir)"
}

puread_ledger_path() {
    printf '%s/actions.jsonl' "$(puread_state_dir)"
}

puread_profile_ledger_path() {
    printf '%s/profile-actions.jsonl' "$(puread_state_dir)"
}

puread_module_version() {
    sed -n 's/^version=//p' "$PUREAD_MODDIR/module.prop" 2>/dev/null | head -n 1
}

puread_auto_apply_marker() {
    PUREAD_VERSION="$(puread_module_version)"
    if [ -z "$PUREAD_VERSION" ]; then
        PUREAD_VERSION="unknown"
    fi
    printf '%s/auto-apply-%s.done' "$(puread_state_dir)" "$PUREAD_VERSION"
}

puread_prepare_runtime_dirs() {
    puread_prepare_state_dir
    puread_prepare_run_dir
    puread_prepare_log_dir
}

puread_prepare_state_dir() {
    mkdir -p "$(puread_state_dir)" 2>/dev/null || true
    chmod 700 "$(puread_state_dir)" 2>/dev/null || true
}

puread_prepare_run_dir() {
    mkdir -p "$(puread_run_dir)" 2>/dev/null || true
    chmod 700 "$(puread_run_dir)" 2>/dev/null || true
}

puread_prepare_log_dir() {
    mkdir -p "$(puread_log_dir)" 2>/dev/null || true
    chmod 700 "$(puread_log_dir)" 2>/dev/null || true
}

puread_apply_module_permissions() {
    if command -v set_perm >/dev/null 2>&1; then
        set_perm "$PUREAD_MODDIR/customize.sh" 0 0 0755 2>/dev/null || true
        set_perm "$PUREAD_MODDIR/service.sh" 0 0 0755 2>/dev/null || true
        set_perm "$PUREAD_MODDIR/action.sh" 0 0 0755 2>/dev/null || true
        set_perm "$PUREAD_MODDIR/uninstall.sh" 0 0 0755 2>/dev/null || true
        if [ -d "$PUREAD_MODDIR/scripts" ]; then
            set_perm_recursive "$PUREAD_MODDIR/scripts" 0 0 0755 0755 2>/dev/null || true
        fi
        if [ -d "$PUREAD_MODDIR/bin" ]; then
            set_perm_recursive "$PUREAD_MODDIR/bin" 0 0 0755 0755 2>/dev/null || true
        fi
    else
        chmod 755 "$PUREAD_MODDIR/customize.sh" "$PUREAD_MODDIR/service.sh" "$PUREAD_MODDIR/action.sh" "$PUREAD_MODDIR/uninstall.sh" 2>/dev/null || true
        chmod 755 "$PUREAD_MODDIR/scripts"/*.sh 2>/dev/null || true
    fi
    if [ -d "$PUREAD_MODDIR/bin" ]; then
        find "$PUREAD_MODDIR/bin" -type d -exec chmod 755 {} \; 2>/dev/null || true
        find "$PUREAD_MODDIR/bin" -type f -name 'puread-*' -exec chmod 755 {} \; 2>/dev/null || true
    fi
}

puread_log() {
    puread_prepare_log_dir
    printf '%s %s\n' "$(puread_now)" "$*" >>"$(puread_log_path)" 2>/dev/null || true
    chmod 600 "$(puread_log_path)" 2>/dev/null || true
}

puread_write_status() {
    puread_prepare_state_dir
    {
        printf 'module_id=%s\n' "$PUREAD_MODULE_ID"
        printf 'status=%s\n' "$1"
        printf 'detail=%s\n' "$2"
        printf 'runtime=%s\n' "$(puread_runtime_name)"
        printf 'abi=%s\n' "$(puread_select_abi)"
        printf 'daemon=%s\n' "$(puread_binary_path puread-daemon)"
        printf 'cli=%s\n' "$(puread_binary_path puread-cli)"
        printf 'state_dir=%s\n' "$(puread_state_dir)"
        printf 'run_dir=%s\n' "$(puread_run_dir)"
        printf 'log_path=%s\n' "$(puread_log_path)"
        printf 'lock_path=%s\n' "$(puread_lock_path)"
        printf 'updated_at=%s\n' "$(puread_now)"
    } >"$(puread_status_path)" 2>/dev/null || true
    chmod 600 "$(puread_status_path)" 2>/dev/null || true
}

puread_pid_running() {
    PUREAD_PID_FILE="$1"
    if [ ! -f "$PUREAD_PID_FILE" ]; then
        return 1
    fi

    PUREAD_PID_VALUE="$(cat "$PUREAD_PID_FILE" 2>/dev/null || printf '%s' "")"
    case "$PUREAD_PID_VALUE" in
        ''|*[!0-9]*)
            return 1
            ;;
    esac

    kill -0 "$PUREAD_PID_VALUE" 2>/dev/null
}

puread_pid_matches_daemon() {
    PUREAD_PID_FILE="$1"
    if ! puread_pid_running "$PUREAD_PID_FILE"; then
        return 1
    fi
    PUREAD_PID_VALUE="$(cat "$PUREAD_PID_FILE" 2>/dev/null || printf '%s' "")"
    PUREAD_CMDLINE="/proc/$PUREAD_PID_VALUE/cmdline"
    if [ ! -r "$PUREAD_CMDLINE" ]; then
        return 1
    fi
    PUREAD_EXPECTED_DAEMON="${PUREAD_DAEMON_BIN:-$(puread_binary_path puread-daemon)}"
    PUREAD_CMD_LINES="$(tr '\000' '\n' <"$PUREAD_CMDLINE" 2>/dev/null || printf '%s' "")"
    PUREAD_CMD_ARG0="$(printf '%s\n' "$PUREAD_CMD_LINES" | sed -n '1p' 2>/dev/null || printf '%s' "")"
    [ "$PUREAD_CMD_ARG0" = "$PUREAD_EXPECTED_DAEMON" ] || return 1
    printf '%s\n' "$PUREAD_CMD_LINES" | grep -Fx "$PUREAD_MODDIR/rules" >/dev/null 2>&1 || return 1
    printf '%s\n' "$PUREAD_CMD_LINES" | grep -Fx "$(puread_state_dir)" >/dev/null 2>&1 || return 1
    printf '%s\n' "$PUREAD_CMD_LINES" | grep -Fx "$(puread_log_path)" >/dev/null 2>&1 || return 1
}
