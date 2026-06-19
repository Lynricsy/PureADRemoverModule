zip_check_with_python() {
    zip_path="$1"
    display_path="${2:-$zip_path}"
    python3 - "$zip_path" "$display_path" <<'PY'
import re
import sys
import zipfile

zip_path = sys.argv[1]
display_path = sys.argv[2]
required_files = {
    "module.prop",
    "customize.sh",
    "service.sh",
    "post-fs-data.sh",
    "uninstall.sh",
    "action.sh",
    "scripts/puread-module-lib.sh",
}
forbidden_name = re.compile(
    r"(^|/)(Example|Host)(/|$)|\.zip$|hosts|host\.sh|mount_hosts|private[_-]?dns|dns|iptables|clash|mihomo|adguardhome|adguard-home|proxy|proxyconfig|ad_reward|ifw",
    re.IGNORECASE,
)

with zipfile.ZipFile(zip_path) as archive:
    bad_member = archive.testzip()
    if bad_member is not None:
        raise SystemExit(f"corrupt zip member: {bad_member}")
    names = set(archive.namelist())

missing = sorted(required_files - names)
if missing:
    raise SystemExit("missing entries: " + ", ".join(missing))

rule_files = sorted(name for name in names if name.startswith("rules/") and name.endswith(".toml"))
if not rule_files:
    raise SystemExit("missing rules/*.toml entries")

daemon_abis = {
    name.split("/")[1]
    for name in names
    if name.startswith("bin/") and name.endswith("/puread-daemon")
}
cli_abis = {
    name.split("/")[1]
    for name in names
    if name.startswith("bin/") and name.endswith("/puread-cli")
}
if not daemon_abis:
    raise SystemExit("missing bin/<abi>/puread-daemon")
if daemon_abis != cli_abis:
    raise SystemExit(f"daemon/cli ABI mismatch: daemon={sorted(daemon_abis)} cli={sorted(cli_abis)}")

forbidden_entries = sorted(name for name in names if forbidden_name.search(name))
if forbidden_entries:
    raise SystemExit("forbidden zip path entries: " + ", ".join(forbidden_entries))

print(f"zip={display_path}")
print(f"entry_count={len(names)}")
print(f"rule_file_count={len(rule_files)}")
print("abis=" + ",".join(sorted(daemon_abis)))
print("required_entries=present")
print("forbidden_path_entries=none")
PY
}

zip_list_entries() {
    zip_path="$1"
    unzip -Z1 "$zip_path"
}

zip_has_entry() {
    entry="$1"
    entries_file="$2"
    grep -Fx "$entry" "$entries_file" >/dev/null 2>&1
}

zip_check_with_unzip() {
    zip_path="$1"
    display_path="${2:-$zip_path}"
    entries_file="$TMP_ROOT/zip-check-entries.txt"
    rule_files="$TMP_ROOT/zip-check-rules.txt"
    daemon_abis="$TMP_ROOT/zip-check-daemon-abis.txt"
    cli_abis="$TMP_ROOT/zip-check-cli-abis.txt"
    forbidden_entries="$TMP_ROOT/zip-check-forbidden.txt"

    unzip -t "$zip_path" >/dev/null || return 1
    zip_list_entries "$zip_path" >"$entries_file" || return 1

    for required in \
        module.prop \
        customize.sh \
        service.sh \
        post-fs-data.sh \
        uninstall.sh \
        action.sh \
        scripts/puread-module-lib.sh
    do
        zip_has_entry "$required" "$entries_file" || {
            printf 'missing entries: %s\n' "$required" >&2
            return 1
        }
    done

    grep '^rules/.*\.toml$' "$entries_file" >"$rule_files" || {
        printf '%s\n' "missing rules/*.toml entries" >&2
        return 1
    }
    sed -n 's#^bin/\([^/][^/]*\)/puread-daemon$#\1#p' "$entries_file" | sort -u >"$daemon_abis"
    sed -n 's#^bin/\([^/][^/]*\)/puread-cli$#\1#p' "$entries_file" | sort -u >"$cli_abis"
    [ -s "$daemon_abis" ] || {
        printf '%s\n' "missing bin/<abi>/puread-daemon" >&2
        return 1
    }
    if ! cmp -s "$daemon_abis" "$cli_abis"; then
        printf '%s' "daemon/cli ABI mismatch: daemon=" >&2
        paste -sd, "$daemon_abis" >&2
        printf '%s' " cli=" >&2
        paste -sd, "$cli_abis" >&2
        return 1
    fi
    grep -Ei '(^|/)(Example|Host)(/|$)|\.zip$|hosts|host\.sh|mount_hosts|private[_-]?dns|dns|iptables|clash|mihomo|adguardhome|adguard-home|proxy|proxyconfig|ad_reward|ifw' "$entries_file" >"$forbidden_entries" || true
    if [ -s "$forbidden_entries" ]; then
        printf '%s' "forbidden zip path entries: " >&2
        paste -sd, "$forbidden_entries" >&2
        return 1
    fi

    printf 'zip=%s\n' "$display_path"
    printf 'entry_count=%s\n' "$(wc -l <"$entries_file" | tr -d ' ')"
    printf 'rule_file_count=%s\n' "$(wc -l <"$rule_files" | tr -d ' ')"
    printf 'abis='
    paste -sd, "$daemon_abis"
    printf '%s\n' "required_entries=present"
    printf '%s\n' "forbidden_path_entries=none"
}

zip_check_snapshot_with_python() {
    zip_path="$1"
    evidence_tmp="$2"
    snapshot="$TMP_ROOT/zip-check-package.zip"
    attempt_output="$TMP_ROOT/zip-check-attempt.txt"
    attempt=1
    rc=1

    while [ "$attempt" -le 3 ]; do
        if {
            rm -f "$snapshot"
            cp "$zip_path" "$snapshot"
            zip_check_with_python "$snapshot" "$zip_path"
        } >"$attempt_output" 2>&1; then
            cat "$attempt_output" >>"$evidence_tmp"
            rm -f "$snapshot" "$attempt_output"
            return 0
        else
            rc=$?
        fi
        attempt=$((attempt + 1))
        sleep 1
    done

    cat "$attempt_output" >>"$evidence_tmp"
    rm -f "$snapshot" "$attempt_output"
    return "$rc"
}

zip_check_snapshot_with_unzip() {
    zip_path="$1"
    evidence_tmp="$2"
    snapshot="$TMP_ROOT/zip-check-package.zip"
    attempt_output="$TMP_ROOT/zip-check-attempt.txt"
    rc=1

    rm -f "$snapshot"
    cp "$zip_path" "$snapshot"
    if zip_check_with_unzip "$snapshot" "$zip_path" >"$attempt_output" 2>&1; then
        cat "$attempt_output" >>"$evidence_tmp"
        rm -f "$snapshot" "$attempt_output"
        return 0
    else
        rc=$?
    fi

    cat "$attempt_output" >>"$evidence_tmp"
    rm -f "$snapshot" "$attempt_output"
    return "$rc"
}

write_zip_check() {
    evidence="$TASK_PREFIX-zip-check.txt"
    evidence_tmp="$(mktemp "${evidence}.tmp.XXXXXX")"
    package_evidence="$TASK_PREFIX-package.txt"

    printf '%s\n' "gate=zip-check" >"$evidence_tmp"

    zip_path="$(sed -n 's/^zip=//p' "$package_evidence" | tail -n 1)"
    if [ -z "$zip_path" ]; then
        printf '%s\n' "result=fail" >>"$evidence_tmp"
        mv "$evidence_tmp" "$evidence"
        fail "package output did not contain zip path; see $package_evidence"
        return
    fi
    if [ ! -f "$zip_path" ]; then
        printf 'zip=%s\n' "$zip_path" >>"$evidence_tmp"
        printf '%s\n' "result=fail" >>"$evidence_tmp"
        mv "$evidence_tmp" "$evidence"
        fail "package zip missing: $zip_path"
        return
    fi

    if command -v python3 >/dev/null 2>&1; then
        if zip_check_snapshot_with_python "$zip_path" "$evidence_tmp"; then
            printf '%s\n' "result=pass" >>"$evidence_tmp"
            mv "$evidence_tmp" "$evidence"
            info "pass: zip check"
        else
            rc=$?
            printf 'exit_code=%s\n' "$rc" >>"$evidence_tmp"
            printf '%s\n' "result=fail" >>"$evidence_tmp"
            mv "$evidence_tmp" "$evidence"
            fail "zip structure check failed; see $evidence"
        fi
        return
    fi

    if command -v unzip >/dev/null 2>&1; then
        if zip_check_snapshot_with_unzip "$zip_path" "$evidence_tmp"; then
            printf '%s\n' "result=pass" >>"$evidence_tmp"
            mv "$evidence_tmp" "$evidence"
            info "pass: zip check with unzip"
        else
            rc=$?
            printf 'exit_code=%s\n' "$rc" >>"$evidence_tmp"
            printf '%s\n' "result=fail" >>"$evidence_tmp"
            mv "$evidence_tmp" "$evidence"
            fail "zip structure check failed; see $evidence"
        fi
        return
    fi

    printf '%s\n' "result=fail" >>"$evidence_tmp"
    mv "$evidence_tmp" "$evidence"
    fail "python3 and unzip are unavailable; cannot verify package zip"
}
