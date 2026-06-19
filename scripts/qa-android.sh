#!/bin/sh
set -eu

EVIDENCE_DIR=".omo/evidence"
ADB_DEVICES_EVIDENCE="$EVIDENCE_DIR/task-27-adb-devices.txt"
DEVICE_UNAVAILABLE_EVIDENCE="$EVIDENCE_DIR/task-27-device-unavailable.txt"
STATIC_EVIDENCE="$EVIDENCE_DIR/task-27-static-compat.txt"
DEVICE_EVIDENCE="$EVIDENCE_DIR/task-27-android-device.txt"

SCRIPT_DIR="$(dirname "$0")"
LIB_DIR="$SCRIPT_DIR/lib"

. "$LIB_DIR/qa-android-common.sh"
. "$LIB_DIR/qa-android-static.sh"
. "$LIB_DIR/qa-android-device.sh"

DRY_RUN=0
SERIAL=""
MODULE_DIR="/data/adb/modules/PureAD"
PROFILE="conservative"
DEVICE_ROOT="/"
SCAN_ROOT="/data"
PUSH_ZIP=""
PUSH_CLI=""
PUSH_DAEMON=""
RUN_DAEMON_CHECK=1
ADB_BIN=""
SELECTED_SERIAL=""
FAIL_COUNT=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --dry-run)
            DRY_RUN=1
            ;;
        --serial)
            shift
            [ "$#" -gt 0 ] || fail_usage "--serial requires a value"
            SERIAL="$1"
            ;;
        --module-dir)
            shift
            [ "$#" -gt 0 ] || fail_usage "--module-dir requires a value"
            MODULE_DIR="$1"
            ;;
        --profile)
            shift
            [ "$#" -gt 0 ] || fail_usage "--profile requires a value"
            PROFILE="$1"
            ;;
        --device-root)
            shift
            [ "$#" -gt 0 ] || fail_usage "--device-root requires a value"
            DEVICE_ROOT="$1"
            ;;
        --scan-root)
            shift
            [ "$#" -gt 0 ] || fail_usage "--scan-root requires a value"
            SCAN_ROOT="$1"
            ;;
        --push-zip)
            shift
            [ "$#" -gt 0 ] || fail_usage "--push-zip requires a value"
            PUSH_ZIP="$1"
            ;;
        --push-cli)
            shift
            [ "$#" -gt 0 ] || fail_usage "--push-cli requires a value"
            PUSH_CLI="$1"
            ;;
        --push-daemon)
            shift
            [ "$#" -gt 0 ] || fail_usage "--push-daemon requires a value"
            PUSH_DAEMON="$1"
            ;;
        --skip-daemon)
            RUN_DAEMON_CHECK=0
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            fail_usage "unknown argument: $1"
            ;;
    esac
    shift
done

mkdir -p "$EVIDENCE_DIR"

record_static_checks
record_adb_devices || DEVICE_AVAILABLE=0
DEVICE_AVAILABLE="${DEVICE_AVAILABLE:-1}"
print_command_plan

if [ "$DRY_RUN" -eq 1 ]; then
    printf '%s\n' "qa_status=plan_only"
    printf '%s\n' "real_device_pass=false"
    if [ "$DEVICE_AVAILABLE" -eq 0 ]; then
        write_device_unavailable "no authorized adb device"
    else
        printf 'device_available=%s\n' "$SELECTED_SERIAL"
        printf '%s\n' "device_run=skipped_by_dry_run"
    fi
    exit 0
fi

if [ "$DEVICE_AVAILABLE" -eq 0 ]; then
    write_device_unavailable "no authorized adb device"
    printf '%s\n' "qa_status=device-unavailable"
    printf '%s\n' "real_device_pass=false"
    exit 3
fi

run_device_smoke
if [ "$FAIL_COUNT" -eq 0 ]; then
    printf '%s\n' "qa_status=real_device_pass"
    printf '%s\n' "real_device_pass=true"
    exit 0
fi

printf '%s\n' "qa_status=real_device_failed"
printf '%s\n' "real_device_pass=false"
exit 1
