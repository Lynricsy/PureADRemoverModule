record_static_checks() {
    : >"$STATIC_EVIDENCE"
    printf '%s\n' "mode=static-script-compatibility" >>"$STATIC_EVIDENCE"
    printf '%s\n' "real_device_pass=false" >>"$STATIC_EVIDENCE"
    printf '%s\n' "ksu=syntax-only" >>"$STATIC_EVIDENCE"
    printf '%s\n' "apatch=syntax-only" >>"$STATIC_EVIDENCE"
    for path in \
        module/customize.sh \
        module/service.sh \
        module/action.sh \
        module/uninstall.sh \
        module/scripts/puread-module-lib.sh \
        scripts/qa-android.sh
    do
        if sh -n "$path" >>"$STATIC_EVIDENCE" 2>&1; then
            printf 'syntax_ok=%s\n' "$path" >>"$STATIC_EVIDENCE"
        else
            printf 'syntax_failed=%s\n' "$path" >>"$STATIC_EVIDENCE"
            FAIL_COUNT=$((FAIL_COUNT + 1))
        fi
    done
    info "static compatibility evidence: $STATIC_EVIDENCE"
}

record_adb_devices() {
    ADB_DEVICES_RC=0
    if ! command -v adb >/dev/null 2>&1; then
        {
            printf '%s\n' "adb=unavailable"
            printf '%s\n' "device-unavailable"
            printf '%s\n' "real_device_pass=false"
        } >"$ADB_DEVICES_EVIDENCE"
        return 1
    fi

    ADB_BIN="$(command -v adb)"
    ADB_DEVICES_OUTPUT="$("$ADB_BIN" devices 2>&1)" || ADB_DEVICES_RC=$?
    {
        printf 'adb_path=%s\n' "$ADB_BIN"
        printf 'adb_devices_rc=%s\n' "$ADB_DEVICES_RC"
        printf '%s\n' "$ADB_DEVICES_OUTPUT"
    } >"$ADB_DEVICES_EVIDENCE"

    if [ "$ADB_DEVICES_RC" -ne 0 ]; then
        printf '%s\n' "device-unavailable" >>"$ADB_DEVICES_EVIDENCE"
        printf '%s\n' "real_device_pass=false" >>"$ADB_DEVICES_EVIDENCE"
        return 1
    fi

    if [ -n "$SERIAL" ]; then
        DEVICE_STATE="$(printf '%s\n' "$ADB_DEVICES_OUTPUT" | awk -v serial="$SERIAL" '$1 == serial { print $2; exit }')"
        if [ "$DEVICE_STATE" = "device" ]; then
            SELECTED_SERIAL="$SERIAL"
            printf 'selected_device=%s\n' "$SELECTED_SERIAL" >>"$ADB_DEVICES_EVIDENCE"
            return 0
        fi
        printf 'requested_serial=%s\n' "$SERIAL" >>"$ADB_DEVICES_EVIDENCE"
        printf 'requested_state=%s\n' "${DEVICE_STATE:-missing}" >>"$ADB_DEVICES_EVIDENCE"
        printf '%s\n' "device-unavailable" >>"$ADB_DEVICES_EVIDENCE"
        printf '%s\n' "real_device_pass=false" >>"$ADB_DEVICES_EVIDENCE"
        return 1
    fi

    SELECTED_SERIAL="$(printf '%s\n' "$ADB_DEVICES_OUTPUT" | awk 'NR > 1 && $2 == "device" { print $1; exit }')"
    if [ -n "$SELECTED_SERIAL" ]; then
        printf 'selected_device=%s\n' "$SELECTED_SERIAL" >>"$ADB_DEVICES_EVIDENCE"
        return 0
    fi

    printf '%s\n' "device-unavailable" >>"$ADB_DEVICES_EVIDENCE"
    printf '%s\n' "real_device_pass=false" >>"$ADB_DEVICES_EVIDENCE"
    return 1
}

write_device_unavailable() {
    reason="$1"
    {
        printf 'result=%s\n' "device-unavailable"
        printf 'reason=%s\n' "$reason"
        printf 'dry_run=%s\n' "$DRY_RUN"
        printf '%s\n' "real_device_pass=false"
        printf 'adb_devices_evidence=%s\n' "$ADB_DEVICES_EVIDENCE"
        printf 'static_evidence=%s\n' "$STATIC_EVIDENCE"
    } >"$DEVICE_UNAVAILABLE_EVIDENCE"
    info "device unavailable evidence: $DEVICE_UNAVAILABLE_EVIDENCE"
}
