write_loc_check() {
    evidence="$TASK_PREFIX-loc.txt"
    : >"$evidence"
    printf '%s\n' "gate=pure-rust-loc" >>"$evidence"
    printf '%s\n' "limit=250" >>"$evidence"

    if command -v git >/dev/null 2>&1; then
        {
            printf '%s\n' "touched_rust_files:"
            git diff --name-only -- '*.rs' 2>/dev/null || true
            git ls-files --others --exclude-standard -- '*.rs' 2>/dev/null || true
        } >>"$evidence"
    fi

    find . \
        -path './.git' -prune -o \
        -path './target' -prune -o \
        -path './Example' -prune -o \
        -path './xtask/upstream-report/target' -prune -o \
        -type f -name '*.rs' -print | sort >"$RUST_FILES"

    total=0
    over=0
    while IFS= read -r file; do
        loc="$(sed '/^[[:space:]]*$/d;/^[[:space:]]*\/\//d' "$file" | wc -l | tr -d ' ')"
        printf '%s\t%s\n' "$loc" "$file" >>"$LOC_ROWS"
        total=$((total + 1))
        if [ "$loc" -gt 250 ]; then
            printf 'OVER\t%s\t%s\n' "$loc" "$file" >>"$LOC_OVER"
            over=$((over + 1))
        fi
    done <"$RUST_FILES"

    {
        printf 'rust_file_count=%s\n' "$total"
        printf '%s\n' "loc_rows:"
        sort -rn "$LOC_ROWS" | sed 's/^/  /'
    } >>"$evidence"

    if [ "$over" -eq 0 ]; then
        printf '%s\n' "result=pass" >>"$evidence"
        info "pass: pure Rust LOC"
    else
        {
            printf '%s\n' "historical_exceptions=none"
            printf '%s\n' "over_limit:"
            sed 's/^/  /' "$LOC_OVER"
            printf '%s\n' "result=fail"
        } >>"$evidence"
        fail "pure Rust LOC check found $over files over 250 LOC; see $evidence"
    fi
}

write_shell_parse_check() {
    evidence="$TASK_PREFIX-shell-parse.txt"
    : >"$evidence"
    printf '%s\n' "gate=shell-parse" >>"$evidence"

    find module scripts -path 'module/bin' -prune -o -type f -name '*.sh' -print | sort >"$SHELL_FILES"
    total=0
    failed=0
    while IFS= read -r file; do
        total=$((total + 1))
        if sh -n "$file" >>"$evidence" 2>&1; then
            printf 'PASS\t%s\n' "$file" >>"$evidence"
        else
            printf 'FAIL\t%s\n' "$file" >>"$evidence"
            failed=$((failed + 1))
        fi
    done <"$SHELL_FILES"

    printf 'shell_file_count=%s\n' "$total" >>"$evidence"
    if [ "$failed" -eq 0 ]; then
        printf '%s\n' "result=pass" >>"$evidence"
        info "pass: shell parse"
    else
        printf 'failed_count=%s\n' "$failed" >>"$evidence"
        printf '%s\n' "result=fail" >>"$evidence"
        fail "shell parse check found $failed failures; see $evidence"
    fi
}
