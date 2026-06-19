#!/bin/sh
set -eu

usage() {
    printf '%s\n' "usage: scripts/package-module.sh [--skip-build]"
    printf '%s\n' ""
    printf '%s\n' "Environment:"
    printf '%s\n' "  PUREAD_DIST_DIR=dist                 Output directory"
    printf '%s\n' "  PUREAD_PACKAGE_NAME=PureAD.zip       Output file name"
    printf '%s\n' "  PUREAD_SKIP_BUILD=1                  Reuse module/bin artifacts"
}

die() {
    printf '%s\n' "error: $*" >&2
    exit 1
}

info() {
    printf '%s\n' "$*"
}

copy_tree_contents() {
    source_dir="$1"
    dest_dir="$2"
    [ -d "$source_dir" ] || die "missing directory: $source_dir"
    mkdir -p "$dest_dir"
    cp -R "$source_dir"/. "$dest_dir"/
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

sanitize_name() {
    printf '%s' "$1" | tr -c 'A-Za-z0-9._-' '_'
}

require_file() {
    [ -f "$PUREAD_STAGE/$1" ] || die "missing package entry: $1"
}

require_native_pairs() {
    found=0
    for daemon_path in "$PUREAD_STAGE"/bin/*/puread-daemon; do
        [ -f "$daemon_path" ] || continue
        abi_dir="$(dirname "$daemon_path")"
        [ -x "$daemon_path" ] || die "daemon is not executable: $daemon_path"
        [ -x "$abi_dir/puread-cli" ] || die "missing executable CLI next to daemon: $abi_dir/puread-cli"
        found=1
    done
    [ "$found" -eq 1 ] || die "no native daemon/CLI pairs staged under bin/<abi>/"
}

require_rules() {
    [ -d "$PUREAD_STAGE/rules" ] || die "missing package entry: rules/"
    if ! find "$PUREAD_STAGE/rules" -type f -name '*.toml' -print -quit | grep -q .; then
        die "no rule TOML files staged under rules/"
    fi
}

write_post_fs_data_entry() {
    cat >"$PUREAD_STAGE/post-fs-data.sh" <<'SH'
#!/system/bin/sh
exit 0
SH
}

create_zip() {
    rm -f "$PUREAD_ZIP"
    if command -v zip >/dev/null 2>&1; then
        (
            cd "$PUREAD_STAGE"
            zip -qr "$PUREAD_ZIP" module.prop customize.sh service.sh post-fs-data.sh uninstall.sh action.sh scripts bin rules
        )
    elif command -v python3 >/dev/null 2>&1; then
        create_zip_with_python
    else
        die "zip and python3 are both unavailable"
    fi
}

PUREAD_SKIP="${PUREAD_SKIP_BUILD:-0}"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --skip-build)
            PUREAD_SKIP=1
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

PUREAD_SCRIPT_DIR="$(dirname "$0")"
PUREAD_ROOT="$(CDPATH= cd "$PUREAD_SCRIPT_DIR/.." && pwd -P)"
PUREAD_PACKAGE_ZIP_LIB="$PUREAD_ROOT/scripts/lib/package-module-zip.sh"
[ -f "$PUREAD_PACKAGE_ZIP_LIB" ] || die "missing package zip library: $PUREAD_PACKAGE_ZIP_LIB"
. "$PUREAD_PACKAGE_ZIP_LIB"
PUREAD_MODULE="$PUREAD_ROOT/module"
PUREAD_DIST="$(absolute_path "${PUREAD_DIST_DIR:-dist}")"
PUREAD_STAGE=""

cleanup_stage() {
    [ -n "${PUREAD_STAGE:-}" ] || return 0
    case "$PUREAD_STAGE" in
        "$PUREAD_DIST"/.puread-module-staging.*)
            [ -d "$PUREAD_STAGE" ] && rm -rf "$PUREAD_STAGE"
            ;;
        *)
            printf '%s\n' "error: refusing to clean unsafe staging path: $PUREAD_STAGE" >&2
            return 1
            ;;
    esac
}

create_stage() {
    PUREAD_STAGE="$(mktemp -d "$PUREAD_DIST/.puread-module-staging.XXXXXX")" || {
        die "failed to create unique staging directory under $PUREAD_DIST"
    }
}

[ -d "$PUREAD_MODULE" ] || die "missing module directory: $PUREAD_MODULE"
[ -d "$PUREAD_ROOT/rules" ] || die "missing root rules directory: $PUREAD_ROOT/rules"

if [ "$PUREAD_SKIP" != "1" ]; then
    "$PUREAD_ROOT/scripts/build-module.sh"
else
    info "build=skipped"
fi

module_id="$(sed -n 's/^id=//p' "$PUREAD_MODULE/module.prop" | head -n 1)"
module_version="$(sed -n 's/^version=//p' "$PUREAD_MODULE/module.prop" | head -n 1)"
[ -n "$module_id" ] || module_id="PureAD"
[ -n "$module_version" ] || module_version="0.0.0"
default_name="$(sanitize_name "$module_id")-$(sanitize_name "$module_version").zip"

mkdir -p "$PUREAD_DIST"
PUREAD_ZIP="$PUREAD_DIST/${PUREAD_PACKAGE_NAME:-$default_name}"

create_stage
trap cleanup_stage EXIT HUP INT TERM

cp "$PUREAD_MODULE/module.prop" "$PUREAD_STAGE/module.prop"
cp "$PUREAD_MODULE/customize.sh" "$PUREAD_STAGE/customize.sh"
cp "$PUREAD_MODULE/service.sh" "$PUREAD_STAGE/service.sh"
cp "$PUREAD_MODULE/uninstall.sh" "$PUREAD_STAGE/uninstall.sh"
cp "$PUREAD_MODULE/action.sh" "$PUREAD_STAGE/action.sh"
write_post_fs_data_entry
copy_tree_contents "$PUREAD_MODULE/scripts" "$PUREAD_STAGE/scripts"
copy_tree_contents "$PUREAD_MODULE/bin" "$PUREAD_STAGE/bin"
mkdir -p "$PUREAD_STAGE/rules"
copy_tree_contents "$PUREAD_MODULE/rules" "$PUREAD_STAGE/rules"
copy_tree_contents "$PUREAD_ROOT/rules" "$PUREAD_STAGE/rules"
rm -f "$PUREAD_STAGE/bin/README.md" "$PUREAD_STAGE/rules/README.md"

chmod 755 "$PUREAD_STAGE/customize.sh" "$PUREAD_STAGE/service.sh" "$PUREAD_STAGE/post-fs-data.sh" "$PUREAD_STAGE/uninstall.sh" "$PUREAD_STAGE/action.sh"
find "$PUREAD_STAGE/scripts" -type f -name '*.sh' | while IFS= read -r script_file; do
    chmod 755 "$script_file"
done
find "$PUREAD_STAGE/bin" -type f -name 'puread-*' | while IFS= read -r binary_file; do
    chmod 755 "$binary_file"
done

require_file "module.prop"
require_file "service.sh"
require_file "post-fs-data.sh"
require_file "uninstall.sh"
require_file "action.sh"
require_file "customize.sh"
require_file "scripts/puread-module-lib.sh"
require_file "scripts/puread-action-lib.sh"
require_native_pairs
require_rules

create_zip
cleanup_stage
PUREAD_STAGE=""

info "zip=$PUREAD_ZIP"
info "done=package-module"
