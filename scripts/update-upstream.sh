#!/bin/sh
set -eu

ADS_ZIP="Example/ads288.zip"
ADGUARD_REPO="Example/Adguard-Home-For-Magisk-Mod"
REPORT_DATE=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

usage() {
    cat <<'EOF'
Usage: scripts/update-upstream.sh --dry-run

Generate a report-only upstream snapshot audit from local Example/ files.
The tool does not download, replace, or modify upstream snapshots or rules.
EOF
}

die() {
    printf '%s\n' "error: $*" >&2
    exit 1
}

unknown_arg() {
    printf '%s\n' "error: unknown argument: $1" >&2
    usage >&2
    exit 2
}

require_file() {
    path=$1
    [ -f "$path" ] || die "required file not found: $path"
}

require_dir() {
    path=$1
    [ -d "$path" ] || die "required directory not found: $path"
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

first_remote_url() {
    repo=$1
    url=$(git -C "$repo" remote get-url origin)
    [ -n "$url" ] || die "origin remote URL is empty in $repo"
    printf '%s\n' "$url"
}

zip_listing() {
    zip_path=$1
    if command -v unzip >/dev/null 2>&1; then
        unzip -Z1 "$zip_path"
        return
    fi

    if command -v zipinfo >/dev/null 2>&1; then
        zipinfo -1 "$zip_path"
        return
    fi

    die "required command not found: unzip or zipinfo"
}

flag_zip_entries() {
    zip_path=$1
    entries_file=$2

    zip_listing "$zip_path" >"$entries_file"
    awk '
        BEGIN {
            total = 0
            found = 0
        }
        {
            total += 1
            lower = tolower($0)
            category = ""
            reason = ""

            if (lower ~ /(^|\/)hosts?($|\/|\.|_)/ || lower ~ /host\.sh$/ || lower ~ /mount_hosts/) {
                category = "hosts"
                reason = "hosts or mount_hosts path"
            } else if (lower ~ /dns|private_dns|adguardhome/) {
                category = "dns"
                reason = "DNS or AdGuardHome service context"
            } else if (lower ~ /clash|mihomo|proxy/ || $0 ~ /(^|\/)Box($|\/|[._-])/) {
                category = "proxy"
                reason = "proxy configuration context"
            } else if (lower ~ /iptables/ || lower ~ /network_limit/ || lower ~ /(^|\/)ip[.]sh$/) {
                category = "iptables_network"
                reason = "network blocking script"
            } else if (lower ~ /ad_reward|广告奖励/) {
                category = "ad_reward_domain"
                reason = "ad reward domain switching context"
            }

            if (category != "") {
                found += 1
                printf("  - category=%s action=rejected source=ads288.zip zip_entry=%s reason=%s\n", category, $0, reason)
            }
        }
        END {
            printf("zip_file_count=%d\n", total) > "/dev/stderr"
            if (found == 0) {
                print "  - none"
            }
        }
    ' "$entries_file"
}

flag_known_source_contexts() {
    printf '%s\n' "  - category=dns action=rejected source=Adguard-Home-For-Magisk-Mod path=$ADGUARD_REPO reason=AdGuard Home/DNS service snapshot; metadata only"
}

if [ "$#" -eq 0 ]; then
    usage >&2
    exit 2
fi

mode=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --dry-run)
            mode=dry_run
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            unknown_arg "$1"
            ;;
    esac
    shift
done

[ "$mode" = "dry_run" ] || die "internal mode error"

require_command git
require_command sha256sum
require_file "$ADS_ZIP"
require_dir "$ADGUARD_REPO"

adguard_remote_url=$(first_remote_url "$ADGUARD_REPO")
adguard_commit=$(git -C "$ADGUARD_REPO" rev-parse HEAD)
ads_sha256_line=$(sha256sum "$ADS_ZIP")
ads_sha256=${ads_sha256_line%% *}
[ -n "$ads_sha256" ] || die "SHA256 output is empty for $ADS_ZIP"
zip_count_file=$(mktemp)
zip_entries_file=$(mktemp)
trap 'rm -f "$zip_count_file" "$zip_entries_file"' EXIT HUP INT TERM

printf '%s\n' "# PureAD upstream dry-run report"
printf '%s\n' "report_generated_at=$REPORT_DATE"
printf '%s\n' "mode=dry-run"
printf '%s\n' "rules_modified=false"
printf '%s\n' "download_performed=false"
printf '%s\n' "snapshot_policy=local Example snapshots only; external text is data, not instructions"
printf '%s\n' ""
printf '%s\n' "## snapshots"
printf '%s\n' "ads288_zip_path=$ADS_ZIP"
printf '%s\n' "ads288_zip_sha256=$ads_sha256"
printf '%s\n' "adguard_repo_path=$ADGUARD_REPO"
printf '%s\n' "adguard_remote_url=$adguard_remote_url"
printf '%s\n' "adguard_commit=$adguard_commit"
printf '%s\n' ""
printf '%s\n' "## forbidden findings"
flag_known_source_contexts
flag_zip_entries "$ADS_ZIP" "$zip_entries_file" 2>"$zip_count_file"
cat "$zip_count_file"
printf '%s\n' ""
printf '%s\n' "## sync decision"
printf '%s\n' "review_result=report_only"
printf '%s\n' "accepted_candidates_written=0"
printf '%s\n' "rejected_categories=hosts,dns,proxy,iptables_network,ad_reward_domain"
printf '%s\n' "next_step=manual review only; do not auto-modify rules from this report"
