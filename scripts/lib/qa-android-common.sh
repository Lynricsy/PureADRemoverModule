usage() {
    cat <<'USAGE'
Usage: scripts/qa-android.sh [options]

Android root module smoke QA for PureAD.

Options:
  --dry-run              Print the command plan and write unavailable artifacts only.
  --serial SERIAL        Use a specific adb serial.
  --module-dir PATH      Installed module path. Default: /data/adb/modules/PureAD.
  --profile NAME         Profile for CLI dry-run. Default: conservative.
  --device-root PATH     Root path passed to puread-cli profile dry-run. Default: /.
  --scan-root PATH       Root path passed to action.sh scan-dry-run. Default: /data.
  --push-zip PATH        Explicitly push a module zip to /data/local/tmp only.
  --push-cli PATH        Explicitly push puread-cli into the installed module bin dir.
  --push-daemon PATH     Explicitly push puread-daemon into the installed module bin dir.
  --skip-daemon          Skip daemon start/stop smoke check.
  -h, --help             Show this help.

The script never flashes or removes a module. Dry-run mode never runs adb shell
actions or adb push; it records the plan and marks real device QA as not run.
USAGE
}

fail_usage() {
    printf '%s\n' "error: $*" >&2
    usage >&2
    exit 2
}

info() {
    printf '%s\n' "info: $*"
}

adb_display() {
    if [ -n "$SELECTED_SERIAL" ]; then
        printf '%s -s %s' "${ADB_BIN:-adb}" "$SELECTED_SERIAL"
    elif [ -n "$SERIAL" ]; then
        printf '%s -s %s' "${ADB_BIN:-adb}" "$SERIAL"
    else
        printf '%s' "${ADB_BIN:-adb}"
    fi
}

shell_quote() {
    printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"
}

quote_args() {
    for arg in "$@"; do
        printf ' '
        shell_quote "$arg"
    done
}

plan_command() {
    printf 'plan: %s\n' "$*"
}

print_command_plan() {
    printf '%s\n' "== command plan =="
    plan_command "adb devices"
    if [ -n "$PUSH_ZIP" ]; then
        plan_command "$(adb_display) push $(shell_quote "$PUSH_ZIP") /data/local/tmp/"
    fi
    if [ -n "$PUSH_CLI" ]; then
        plan_command "$(adb_display) push $(shell_quote "$PUSH_CLI") $MODULE_DIR/bin/__qa__/puread-cli"
        plan_command "$(adb_display) shell su -c 'chmod 755 $MODULE_DIR/bin/__qa__/puread-cli'"
    fi
    if [ -n "$PUSH_DAEMON" ]; then
        plan_command "$(adb_display) push $(shell_quote "$PUSH_DAEMON") $MODULE_DIR/bin/__qa__/puread-daemon"
        plan_command "$(adb_display) shell su -c 'chmod 755 $MODULE_DIR/bin/__qa__/puread-daemon'"
    fi
    plan_command "$(adb_display) shell dumpsys batterystats"
    plan_command "$(adb_display) shell dumpsys alarm"
    plan_command "$(adb_display) shell dumpsys deviceidle"
    plan_command "$(adb_display) shell top -b -n 1"
    plan_command "$(adb_display) shell su -c 'sh $MODULE_DIR/action.sh status'"
    plan_command "$(adb_display) shell su -c 'PUREAD_SCAN_ROOT=$SCAN_ROOT sh $MODULE_DIR/action.sh scan-dry-run'"
    plan_command "$(adb_display) shell su -c '$MODULE_DIR/bin/<abi>/puread-cli apply-profile $PROFILE --dry-run --rules $MODULE_DIR/rules --root $DEVICE_ROOT --module-root $MODULE_DIR'"
    if [ "$RUN_DAEMON_CHECK" -eq 1 ]; then
        plan_command "$(adb_display) shell su -c 'PUREAD_DRY_RUN=1 sh $MODULE_DIR/service.sh'"
        plan_command "$(adb_display) shell su -c 'check $MODULE_DIR/run/puread-daemon.pid and stop only a pid started by this QA run'"
    fi
    printf '%s\n' "== end command plan =="
}
