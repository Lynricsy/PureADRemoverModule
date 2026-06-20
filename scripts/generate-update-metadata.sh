#!/bin/sh
set -eu

usage() {
    printf '%s\n' "usage: scripts/generate-update-metadata.sh --zip ZIP --tag TAG --output-dir DIR"
    printf '%s\n' ""
    printf '%s\n' "Environment:"
    printf '%s\n' "  PUREAD_RELEASE_ABIS=\"arm64-v8a armeabi-v7a x86_64 x86\""
}

die() {
    printf '%s\n' "error: $*" >&2
    exit 1
}

absolute_path() {
    case "$1" in
        /*)
            printf '%s' "$1"
            ;;
        *)
            printf '%s/%s' "$PUREAD_ROOT" "$1"
            ;;
    esac
}

zip_path=""
release_tag=""
output_dir=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --zip)
            [ "$#" -ge 2 ] || die "--zip requires a value"
            zip_path="$2"
            shift
            ;;
        --tag)
            [ "$#" -ge 2 ] || die "--tag requires a value"
            release_tag="$2"
            shift
            ;;
        --output-dir)
            [ "$#" -ge 2 ] || die "--output-dir requires a value"
            output_dir="$2"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "unknown argument: $1"
            ;;
    esac
    shift
done

[ -n "$zip_path" ] || die "--zip is required"
[ -n "$release_tag" ] || die "--tag is required"
[ -n "$output_dir" ] || die "--output-dir is required"

PUREAD_SCRIPT_DIR="$(dirname "$0")"
PUREAD_ROOT="$(CDPATH= cd "$PUREAD_SCRIPT_DIR/.." && pwd -P)"
PUREAD_MODULE_PROP="$PUREAD_ROOT/module/module.prop"
PUREAD_OWNER_REPO="Lynricsy/PureADRemoverModule"

zip_path="$(absolute_path "$zip_path")"
output_dir="$(absolute_path "$output_dir")"
[ -f "$zip_path" ] || die "missing release zip: $zip_path"
[ -f "$PUREAD_MODULE_PROP" ] || die "missing module.prop: $PUREAD_MODULE_PROP"

module_id="$(sed -n 's/^id=//p' "$PUREAD_MODULE_PROP" | head -n 1)"
version="$(sed -n 's/^version=//p' "$PUREAD_MODULE_PROP" | head -n 1)"
version_code="$(sed -n 's/^versionCode=//p' "$PUREAD_MODULE_PROP" | head -n 1)"
update_json="$(sed -n 's/^updateJson=//p' "$PUREAD_MODULE_PROP" | head -n 1)"

[ -n "$module_id" ] || die "module.prop is missing id"
[ -n "$version" ] || die "module.prop is missing version"
[ -n "$version_code" ] || die "module.prop is missing versionCode"
[ -n "$update_json" ] || die "module.prop is missing updateJson"

case "$version_code" in
    *[!0-9]*|"")
        die "versionCode must be a decimal integer"
        ;;
esac

case "$release_tag" in
    v*)
        ;;
    *)
        die "release tag must start with v"
        ;;
esac

expected_latest="https://github.com/$PUREAD_OWNER_REPO/releases/latest/download/update.json"
[ "$update_json" = "$expected_latest" ] || {
    die "module.prop updateJson must be $expected_latest"
}

mkdir -p "$output_dir"

zip_name="$(basename "$zip_path")"
zip_url="https://github.com/$PUREAD_OWNER_REPO/releases/download/$release_tag/$zip_name"
changelog_url="https://github.com/$PUREAD_OWNER_REPO/releases/download/$release_tag/changelog.md"
sha256="$(sha256sum "$zip_path" | awk '{print $1}')"
abis="${PUREAD_RELEASE_ABIS:-unknown}"

python3 - "$output_dir/update.json" "$version" "$version_code" "$zip_url" "$changelog_url" <<'PY'
import json
import sys

output, version, version_code, zip_url, changelog_url = sys.argv[1:]
payload = {
    "version": version,
    "versionCode": int(version_code),
    "zipUrl": zip_url,
    "changelog": changelog_url,
}
with open(output, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, ensure_ascii=False, indent=2)
    handle.write("\n")
PY

{
    printf '# PureAD %s\n\n' "$version"
    printf -- '- Version code: `%s`\n' "$version_code"
    printf -- '- Release tag: `%s`\n' "$release_tag"
    printf -- '- Module zip: `%s`\n' "$zip_name"
    printf -- '- SHA256: `%s`\n' "$sha256"
    printf -- '- Built Android ABIs: `%s`\n' "$abis"
    printf -- '- Default auto profiles: `conservative sdk_cache sqlite`\n'
    printf -- '- AppOps, component and ROM profiles remain explicit/manual unless `PUREAD_AUTO_PROFILES` is overridden.\n'
} >"$output_dir/changelog.md"

printf 'update_json=%s\n' "$output_dir/update.json"
printf 'changelog=%s\n' "$output_dir/changelog.md"
