FORBIDDEN_PATTERN='hosts|host\.sh|mount_hosts|private[_-]?dns|dns|iptables|clash|mihomo|adguardhome|adguard-home|proxy|proxyconfig|domain|ad_reward|ifw|zygisk|root_hide|root[-_ ]?hiding|denylist|shamiko'

is_doc_path() {
    case "$1" in
        AGENTS.md|README.md|*/README.md|*.md|.omo/plans/*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

is_test_path() {
    case "$1" in
        */tests/*|*/fixtures/*|*_tests.rs|*/test_support.rs|*/support/*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

is_rule_note_or_source_match() {
    path="$1"
    text="$2"

    case "$path" in
        rules/*)
            printf '%s\n' "$text" | grep -Eq '^[[:space:]]*(notes|source|source_file|zip_entry|source_line_or_pattern)[[:space:]]='
            ;;
        *)
            return 1
            ;;
    esac
}

is_sdk_component_proxy_class_match() {
    path="$1"
    text="$2"

    case "$path" in
        rules/components/*)
            printf '%s\n' "$text" | grep -Eiq '^[[:space:]]*target_component[[:space:]]=.*[.]proxy[.]'
            ;;
        *)
            return 1
            ;;
    esac
}

is_settings_rejection_guard_match() {
    path="$1"
    text="$2"

    [ "$path" = "crates/puread-android/src/command_runner/settings.rs" ] || return 1
    printf '%s\n' "$text" | grep -Eq 'private_dns_mode|private_dns_specifier|contains\("dns"\)|contains\("host"\)|contains\("proxy"\)|out of scope'
}

is_package_metadata_boundary_match() {
    path="$1"
    text="$2"

    case "$path" in
        crates/*/Cargo.toml)
            printf '%s\n' "$text" | grep -Eq '^[[:space:]]*description[[:space:]]=.*non-domain'
            ;;
        *)
            return 1
            ;;
    esac
}

classify_forbidden_match() {
    path="$1"
    text="$2"

    if is_doc_path "$path"; then
        printf '%s' "allowed:documentation"
        return 0
    fi
    if is_test_path "$path"; then
        printf '%s' "allowed:test_or_fixture"
        return 0
    fi
    case "$path" in
        scripts/verify-local.sh|scripts/lib/verify-local-*.sh)
            printf '%s' "allowed:verification_grep_guard"
            return 0
            ;;
        crates/puread-rules/src/parse.rs|crates/puread-rules/src/category.rs)
            printf '%s' "allowed:rule_rejection_guard"
            return 0
            ;;
        xtask/upstream-report/src/*)
            printf '%s' "allowed:upstream_audit_classifier"
            return 0
            ;;
    esac
    if is_settings_rejection_guard_match "$path" "$text"; then
        printf '%s' "allowed:settings_rejection_guard"
        return 0
    fi
    if is_package_metadata_boundary_match "$path" "$text"; then
        printf '%s' "allowed:package_metadata_boundary"
        return 0
    fi
    if is_rule_note_or_source_match "$path" "$text"; then
        printf '%s' "allowed:rule_notes_or_provenance"
        return 0
    fi
    if is_sdk_component_proxy_class_match "$path" "$text"; then
        printf '%s' "allowed:sdk_component_class_name"
        return 0
    fi

    printf '%s' "blocked:production_capability_context"
    return 1
}

scan_forbidden_path() {
    path="$1"

    [ -e "$path" ] || return 0
    status=0
    rg -n -i --with-filename "$FORBIDDEN_PATTERN" "$path" >>"$FORBIDDEN_MATCHES" 2>>"$FORBIDDEN_RG_ERRORS" || status=$?
    if [ "$status" -ne 0 ] && [ "$status" -ne 1 ]; then
        fail "forbidden scan rg failed for $path with exit $status"
    fi
}

write_forbidden_scan() {
    if ! command -v rg >/dev/null 2>&1; then
        fail "forbidden scan requires ripgrep (rg)"
        return
    fi

    {
        printf '%s\n' "gate=forbidden-scan"
        printf '%s\n' "policy=production execution paths must not implement DNS/hosts/proxy/iptables/private-DNS/IFW/root-hiding capabilities"
        printf '%s\n' "allowed_contexts=documentation,test_or_fixture,verification_grep_guard,rule_rejection_guard,upstream_audit_classifier,settings_rejection_guard,package_metadata_boundary,rule_notes_or_provenance,sdk_component_class_name"
    } >"$FORBIDDEN_EVIDENCE"

    : >"$FORBIDDEN_MATCHES"
    : >"$FORBIDDEN_RG_ERRORS"
    for path in \
        AGENTS.md \
        README.md \
        module \
        scripts \
        rules \
        crates \
        xtask/upstream-report/src \
        xtask/upstream-report/tests
    do
        scan_forbidden_path "$path"
    done

    if [ -s "$FORBIDDEN_RG_ERRORS" ]; then
        {
            printf '%s\n' "rg_errors:"
            sed 's/^/  /' "$FORBIDDEN_RG_ERRORS"
        } >>"$FORBIDDEN_EVIDENCE"
    fi

    if [ ! -s "$FORBIDDEN_MATCHES" ]; then
        printf '%s\n' "result=pass" >>"$FORBIDDEN_EVIDENCE"
        printf '%s\n' "matches=0" >>"$FORBIDDEN_EVIDENCE"
        info "pass: forbidden scan"
        return
    fi

    blocked=0
    allowed=0
    while IFS= read -r match; do
        path="${match%%:*}"
        rest="${match#*:}"
        line_no="${rest%%:*}"
        text="${rest#*:}"
        reason="$(classify_forbidden_match "$path" "$text")" || {
            printf 'BLOCKED\t%s\t%s\n' "$reason" "$match" >>"$FORBIDDEN_EVIDENCE"
            blocked=$((blocked + 1))
            continue
        }
        printf 'ALLOWED\t%s\t%s:%s:%s\n' "$reason" "$path" "$line_no" "$text" >>"$FORBIDDEN_EVIDENCE"
        allowed=$((allowed + 1))
    done <"$FORBIDDEN_MATCHES"

    printf 'allowed_matches=%s\n' "$allowed" >>"$FORBIDDEN_EVIDENCE"
    printf 'blocked_matches=%s\n' "$blocked" >>"$FORBIDDEN_EVIDENCE"
    if [ "$blocked" -eq 0 ]; then
        printf '%s\n' "result=pass" >>"$FORBIDDEN_EVIDENCE"
        info "pass: forbidden scan"
    else
        printf '%s\n' "result=fail" >>"$FORBIDDEN_EVIDENCE"
        fail "forbidden scan found $blocked blocked matches; see $FORBIDDEN_EVIDENCE"
    fi
}
