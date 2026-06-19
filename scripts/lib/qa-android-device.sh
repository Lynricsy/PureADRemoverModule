adb_device() {
    if [ -n "$SELECTED_SERIAL" ]; then
        "$ADB_BIN" -s "$SELECTED_SERIAL" "$@"
    else
        "$ADB_BIN" "$@"
    fi
}

append_device_header() {
    label="$1"
    shift
    {
        printf '\n## %s\n' "$label"
        printf 'command=%s' "$(adb_display)"
        quote_args "$@"
        printf '\n'
    } >>"$DEVICE_EVIDENCE"
}

run_adb_capture() {
    label="$1"
    shift
    append_device_header "$label" "$@"
    if adb_device "$@" >>"$DEVICE_EVIDENCE" 2>&1; then
        printf 'result=%s\n' "ok" >>"$DEVICE_EVIDENCE"
        return 0
    else
        rc=$?
        printf 'result=failed rc=%s\n' "$rc" >>"$DEVICE_EVIDENCE"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 0
    fi
}

run_adb_shell_capture() {
    label="$1"
    shift
    run_adb_capture "$label" shell "$@"
}

remote_basename() {
    base="${1##*/}"
    [ -n "$base" ] || base="puread-artifact"
    printf '%s' "$base"
}

validate_local_file() {
    label="$1"
    path="$2"
    if [ ! -f "$path" ]; then
        printf 'missing_%s=%s\n' "$label" "$path" >>"$DEVICE_EVIDENCE"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi
    return 0
}

run_push_steps() {
    if [ -n "$PUSH_ZIP" ] && validate_local_file "zip" "$PUSH_ZIP"; then
        run_adb_capture "push module zip to temporary storage" push "$PUSH_ZIP" "/data/local/tmp/$(remote_basename "$PUSH_ZIP")"
    fi
    if [ -n "$PUSH_CLI" ] && validate_local_file "cli" "$PUSH_CLI"; then
        run_adb_shell_capture "prepare CLI binary directory" su -c "mkdir -p '$MODULE_DIR/bin/__qa__'"
        run_adb_capture "push CLI binary to staging directory" push "$PUSH_CLI" "$MODULE_DIR/bin/__qa__/puread-cli"
        run_adb_shell_capture "mark CLI binary executable" su -c "chmod 755 '$MODULE_DIR/bin/__qa__/puread-cli'"
    fi
    if [ -n "$PUSH_DAEMON" ] && validate_local_file "daemon" "$PUSH_DAEMON"; then
        run_adb_shell_capture "prepare daemon binary directory" su -c "mkdir -p '$MODULE_DIR/bin/__qa__'"
        run_adb_capture "push daemon binary to staging directory" push "$PUSH_DAEMON" "$MODULE_DIR/bin/__qa__/puread-daemon"
        run_adb_shell_capture "mark daemon binary executable" su -c "chmod 755 '$MODULE_DIR/bin/__qa__/puread-daemon'"
    fi
}

remote_pid_snapshot() {
    run_adb_shell_capture "$1" su -c "if [ -f '$MODULE_DIR/run/puread-daemon.pid' ]; then cat '$MODULE_DIR/run/puread-daemon.pid'; else printf '%s\n' missing; fi"
}

run_daemon_smoke() {
    [ "$RUN_DAEMON_CHECK" -eq 1 ] || return 0
    BEFORE_PID="$(adb_device shell su -c "if [ -f '$MODULE_DIR/run/puread-daemon.pid' ]; then cat '$MODULE_DIR/run/puread-daemon.pid'; fi" 2>/dev/null || true)"
    remote_pid_snapshot "daemon pid before service"
    run_adb_shell_capture "daemon dry-run service start" su -c "PUREAD_DRY_RUN=1 sh '$MODULE_DIR/service.sh'"
    AFTER_PID="$(adb_device shell su -c "if [ -f '$MODULE_DIR/run/puread-daemon.pid' ]; then cat '$MODULE_DIR/run/puread-daemon.pid'; fi" 2>/dev/null || true)"
    remote_pid_snapshot "daemon pid after service"
    run_adb_shell_capture "daemon status after service" su -c "if [ -f '$MODULE_DIR/state/status.env' ]; then sed -n '1,120p' '$MODULE_DIR/state/status.env'; else printf '%s\n' status_file_missing; fi"
    STOP_PID=""
    case "$AFTER_PID" in
        ''|*[!0-9]*)
            STOP_PID=""
            ;;
        *)
            STOP_PID="$AFTER_PID"
            ;;
    esac
    if [ -n "$STOP_PID" ] && [ "$STOP_PID" != "$BEFORE_PID" ]; then
        run_adb_shell_capture "daemon stop for qa-started pid" su -c "kill -TERM '$STOP_PID' 2>/dev/null || true"
    else
        printf '\n## daemon stop for qa-started pid\nresult=skipped no-new-pid\n' >>"$DEVICE_EVIDENCE"
    fi
}

remote_cli_or_unavailable() {
    printf '%s' "for abi in __qa__ arm64-v8a armeabi-v7a x86_64 x86 riscv64; do candidate='$MODULE_DIR/bin/'\"\$abi\"'/puread-cli'; if [ -x \"\$candidate\" ]; then exec \"\$candidate\""
    quote_args "$@"
    printf '%s' "; fi; done; printf '%s\n' cli_unavailable; exit 11"
}

run_device_smoke() {
    : >"$DEVICE_EVIDENCE"
    {
        printf 'selected_device=%s\n' "$SELECTED_SERIAL"
        printf 'module_dir=%s\n' "$MODULE_DIR"
        printf 'profile=%s\n' "$PROFILE"
        printf 'dry_run=%s\n' "$DRY_RUN"
    } >>"$DEVICE_EVIDENCE"

    run_push_steps
    run_adb_capture "adb devices" devices
    run_adb_shell_capture "android api properties" getprop ro.build.version.sdk
    run_adb_shell_capture "runtime abi" getprop ro.product.cpu.abi
    run_adb_shell_capture "power batterystats" dumpsys batterystats
    run_adb_shell_capture "power alarm" dumpsys alarm
    run_adb_shell_capture "power deviceidle" dumpsys deviceidle
    run_adb_shell_capture "process top sample" top -b -n 1
    run_adb_shell_capture "module status action" su -c "sh '$MODULE_DIR/action.sh' status"
    run_adb_shell_capture "module scan dry-run action" su -c "PUREAD_SCAN_ROOT='$SCAN_ROOT' sh '$MODULE_DIR/action.sh' scan-dry-run"
    run_adb_shell_capture "native CLI status" su -c "$(remote_cli_or_unavailable status --rules "$MODULE_DIR/rules" --root "$DEVICE_ROOT" --module-root "$MODULE_DIR")"
    run_adb_shell_capture "native CLI profile dry-run" su -c "$(remote_cli_or_unavailable apply-profile "$PROFILE" --dry-run --rules "$MODULE_DIR/rules" --root "$DEVICE_ROOT" --module-root "$MODULE_DIR")"
    run_daemon_smoke

    {
        printf '\n## summary\n'
        printf 'fail_count=%s\n' "$FAIL_COUNT"
        if [ "$FAIL_COUNT" -eq 0 ]; then
            printf '%s\n' "real_device_pass=true"
        else
            printf '%s\n' "real_device_pass=false"
        fi
    } >>"$DEVICE_EVIDENCE"
    info "device evidence: $DEVICE_EVIDENCE"
}
