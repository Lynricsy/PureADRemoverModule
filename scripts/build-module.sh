#!/bin/sh
set -eu

usage() {
    printf '%s\n' "usage: scripts/build-module.sh [--dry-run]"
    printf '%s\n' ""
    printf '%s\n' "Environment:"
    printf '%s\n' "  PUREAD_BUILD_PROFILE=debug|release     Cargo profile, default: debug"
    printf '%s\n' "  PUREAD_ANDROID_ABIS='arm64-v8a x86_64' Build Android ABI targets"
    printf '%s\n' "  PUREAD_TARGET_ARM64_V8A=triple         Override target triple"
    printf '%s\n' "  PUREAD_TARGET_ARMEABI_V7A=triple       Override target triple"
    printf '%s\n' "  PUREAD_TARGET_X86_64=triple            Override target triple"
    printf '%s\n' "  PUREAD_TARGET_X86=triple               Override target triple"
    printf '%s\n' "  PUREAD_TARGET_RISCV64=triple           Override target triple"
}

die() {
    printf '%s\n' "error: $*" >&2
    exit 1
}

info() {
    printf '%s\n' "$*"
}

host_abi() {
    machine="$(uname -m 2>/dev/null || printf '%s' unknown)"
    case "$machine" in
        aarch64|arm64)
            printf '%s' "arm64-v8a"
            ;;
        armv7l|armv8l)
            printf '%s' "armeabi-v7a"
            ;;
        x86_64|amd64)
            printf '%s' "x86_64"
            ;;
        i386|i686)
            printf '%s' "x86"
            ;;
        riscv64)
            printf '%s' "riscv64"
            ;;
        *)
            die "unsupported host machine for fixture ABI: $machine"
            ;;
    esac
}

validate_abi() {
    case "$1" in
        arm64-v8a|armeabi-v7a|x86_64|x86|riscv64)
            return 0
            ;;
        *)
            die "unsupported ABI: $1"
            ;;
    esac
}

abi_to_target() {
    case "$1" in
        arm64-v8a)
            printf '%s' "${PUREAD_TARGET_ARM64_V8A:-aarch64-linux-android}"
            ;;
        armeabi-v7a)
            printf '%s' "${PUREAD_TARGET_ARMEABI_V7A:-armv7-linux-androideabi}"
            ;;
        x86_64)
            printf '%s' "${PUREAD_TARGET_X86_64:-x86_64-linux-android}"
            ;;
        x86)
            printf '%s' "${PUREAD_TARGET_X86:-i686-linux-android}"
            ;;
        riscv64)
            printf '%s' "${PUREAD_TARGET_RISCV64:-riscv64-linux-android}"
            ;;
        *)
            die "unsupported ABI: $1"
            ;;
    esac
}

profile_dir() {
    if [ "$PUREAD_PROFILE" = "release" ]; then
        printf '%s' "release"
    else
        printf '%s' "debug"
    fi
}

source_dir_for_abi() {
    if [ "$PUREAD_BUILD_MODE" = "host-fixture" ]; then
        printf '%s/target/%s' "$PUREAD_ROOT" "$(profile_dir)"
    else
        printf '%s/target/%s/%s' "$PUREAD_ROOT" "$(abi_to_target "$1")" "$(profile_dir)"
    fi
}

print_copy_plan() {
    abi="$1"
    source_dir="$(source_dir_for_abi "$abi")"
    dest_dir="$PUREAD_ROOT/module/bin/$abi"
    info "plan: copy $source_dir/puread-daemon -> $dest_dir/puread-daemon"
    info "plan: copy $source_dir/puread-cli -> $dest_dir/puread-cli"
}

build_host() {
    if [ "$PUREAD_PROFILE" = "release" ]; then
        cargo build --locked --release --package puread-cli --package puread-daemon
    else
        cargo build --locked --package puread-cli --package puread-daemon
    fi
}

build_android_abi() {
    target="$(abi_to_target "$1")"
    if [ "$PUREAD_PROFILE" = "release" ]; then
        cargo build --locked --release --target "$target" --package puread-cli --package puread-daemon
    else
        cargo build --locked --target "$target" --package puread-cli --package puread-daemon
    fi
}

copy_abi_artifacts() {
    abi="$1"
    source_dir="$(source_dir_for_abi "$abi")"
    dest_dir="$PUREAD_ROOT/module/bin/$abi"

    [ -x "$source_dir/puread-daemon" ] || die "missing built daemon: $source_dir/puread-daemon"
    [ -x "$source_dir/puread-cli" ] || die "missing built CLI: $source_dir/puread-cli"

    mkdir -p "$dest_dir"
    cp "$source_dir/puread-daemon" "$dest_dir/puread-daemon"
    cp "$source_dir/puread-cli" "$dest_dir/puread-cli"
    chmod 755 "$dest_dir/puread-daemon" "$dest_dir/puread-cli"

    info "copied: $dest_dir/puread-daemon"
    info "copied: $dest_dir/puread-cli"
}

PUREAD_DRY_RUN=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --dry-run)
            PUREAD_DRY_RUN=1
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
PUREAD_PROFILE="${PUREAD_BUILD_PROFILE:-debug}"
cd "$PUREAD_ROOT"

case "$PUREAD_PROFILE" in
    debug|release)
        ;;
    *)
        die "PUREAD_BUILD_PROFILE must be debug or release"
        ;;
esac

if [ -n "${PUREAD_ANDROID_ABIS:-}" ]; then
    PUREAD_BUILD_MODE="android-target"
    PUREAD_ABIS="$(printf '%s' "$PUREAD_ANDROID_ABIS" | tr ',' ' ')"
else
    PUREAD_BUILD_MODE="host-fixture"
    PUREAD_ABIS="$(host_abi)"
fi

[ -n "$PUREAD_ABIS" ] || die "no ABI selected"

info "PureAD build-module"
info "root=$PUREAD_ROOT"
info "profile=$PUREAD_PROFILE"
info "mode=$PUREAD_BUILD_MODE"
info "abis=$PUREAD_ABIS"

if [ "$PUREAD_BUILD_MODE" = "host-fixture" ]; then
    info "note=PUREAD_ANDROID_ABIS is unset; this builds current-host fixture binaries for package layout validation only."
    info "note=The resulting binaries are not Android ABI verified."
fi

for abi in $PUREAD_ABIS; do
    validate_abi "$abi"
done

if [ "$PUREAD_DRY_RUN" -eq 1 ]; then
    if [ "$PUREAD_BUILD_MODE" = "host-fixture" ]; then
        if [ "$PUREAD_PROFILE" = "release" ]; then
            info "plan: cargo build --locked --release --package puread-cli --package puread-daemon"
        else
            info "plan: cargo build --locked --package puread-cli --package puread-daemon"
        fi
    fi
    for abi in $PUREAD_ABIS; do
        if [ "$PUREAD_BUILD_MODE" = "android-target" ]; then
            target="$(abi_to_target "$abi")"
            if [ "$PUREAD_PROFILE" = "release" ]; then
                info "plan: cargo build --locked --release --target $target --package puread-cli --package puread-daemon"
            else
                info "plan: cargo build --locked --target $target --package puread-cli --package puread-daemon"
            fi
        fi
        print_copy_plan "$abi"
    done
    info "dry_run=1"
    exit 0
fi

command -v cargo >/dev/null 2>&1 || die "cargo not found"

if [ "$PUREAD_BUILD_MODE" = "host-fixture" ]; then
    build_host
    for abi in $PUREAD_ABIS; do
        copy_abi_artifacts "$abi"
    done
else
    for abi in $PUREAD_ABIS; do
        build_android_abi "$abi"
        copy_abi_artifacts "$abi"
    done
fi

info "done=build-module"
