#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

usage() {
    cat <<'EOF'
Usage: ./package.sh [--target <triple>] [--profile <profile>] [--output-dir <dir>] [--offline]

Builds skillctrl and skillctrl-desktop and packages them into a versioned archive under dist/.

Options:
  --target <triple>     Rust target triple to build for. Defaults to the host target.
  --profile <profile>   Cargo profile to use. Defaults to release.
  --output-dir <dir>    Directory for packaged artifacts. Defaults to dist.
  --offline             Pass --offline to cargo build.
  -h, --help            Show this help message.
EOF
}

require_cmd() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "error: required command not found: $cmd" >&2
        exit 1
    fi
}

write_sha256() {
    local file="$1"
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{ print $1 }'
    elif command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{ print $1 }'
    else
        return 1
    fi
}

workspace_version() {
    awk -F'"' '
        /^\[workspace\.package\]/ { in_section=1; next }
        /^\[/ && in_section { exit }
        in_section && /^version = / { print $2; exit }
    ' "$ROOT_DIR/Cargo.toml"
}

require_cmd cargo
require_cmd npm
require_cmd tar
require_cmd rustc

TARGET_TRIPLE="$(rustc -vV | awk '/^host:/ { print $2 }')"
PROFILE="release"
OUTPUT_DIR="dist"
OFFLINE=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --target)
            TARGET_TRIPLE="${2:-}"
            shift 2
            ;;
        --profile)
            PROFILE="${2:-}"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="${2:-}"
            shift 2
            ;;
        --offline)
            OFFLINE=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [ -z "$TARGET_TRIPLE" ] || [ -z "$PROFILE" ] || [ -z "$OUTPUT_DIR" ]; then
    echo "error: target, profile, and output directory must not be empty" >&2
    exit 1
fi

VERSION="$(workspace_version)"
if [ -z "$VERSION" ]; then
    echo "error: failed to read workspace version from Cargo.toml" >&2
    exit 1
fi

DESKTOP_DIR="$ROOT_DIR/skillctrl-desktop"
if [ ! -d "$DESKTOP_DIR" ]; then
    echo "error: desktop app directory not found: $DESKTOP_DIR" >&2
    exit 1
fi

echo "Preparing skillctrl-desktop frontend..."
if [ ! -d "$DESKTOP_DIR/node_modules" ]; then
    NPM_INSTALL_ARGS=(ci --no-fund --no-audit)
    if [ "$OFFLINE" -eq 1 ]; then
        NPM_INSTALL_ARGS+=(--offline)
    fi
    (
        cd "$DESKTOP_DIR"
        npm "${NPM_INSTALL_ARGS[@]}"
    )
fi
(
    cd "$DESKTOP_DIR"
    npm run build
)

BIN_NAMES=("skillctrl" "skillctrl-desktop")
BIN_EXT=""
if [[ "$TARGET_TRIPLE" == *windows* ]]; then
    BIN_EXT=".exe"
fi

BUILD_ARGS=(build --locked --target "$TARGET_TRIPLE")
for bin_name in "${BIN_NAMES[@]}"; do
    BUILD_ARGS+=(--package "$bin_name")
done
if [ "$PROFILE" = "release" ]; then
    BUILD_ARGS+=(--release)
else
    BUILD_ARGS+=(--profile "$PROFILE")
fi
if [ "$OFFLINE" -eq 1 ]; then
    BUILD_ARGS+=(--offline)
fi

echo "Building ${BIN_NAMES[*]} ${VERSION} for ${TARGET_TRIPLE} (${PROFILE})..."
cargo "${BUILD_ARGS[@]}"

BINARY_PATHS=()
for bin_name in "${BIN_NAMES[@]}"; do
    binary_path="$ROOT_DIR/target/$TARGET_TRIPLE/$PROFILE/$bin_name$BIN_EXT"
    if [ ! -f "$binary_path" ]; then
        echo "error: expected binary not found: $binary_path" >&2
        exit 1
    fi
    BINARY_PATHS+=("$binary_path")
done

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"

PACKAGE_BASENAME="skillctrl-v${VERSION}-${TARGET_TRIPLE}"
ARCHIVE_PATH="$OUTPUT_DIR/${PACKAGE_BASENAME}.tar.gz"
CHECKSUM_PATH="${ARCHIVE_PATH}.sha256"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/skillctrl-package.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

PACKAGE_DIR="$WORK_DIR/$PACKAGE_BASENAME"
mkdir -p "$PACKAGE_DIR"

for index in "${!BIN_NAMES[@]}"; do
    cp "${BINARY_PATHS[$index]}" "$PACKAGE_DIR/${BIN_NAMES[$index]}$BIN_EXT"
done

if [ -f "$ROOT_DIR/README.md" ]; then
    cp "$ROOT_DIR/README.md" "$PACKAGE_DIR/"
fi
if [ -f "$ROOT_DIR/USER_GUIDE.md" ]; then
    cp "$ROOT_DIR/USER_GUIDE.md" "$PACKAGE_DIR/"
fi
if [ -f "$ROOT_DIR/LICENSE-Apache-2.0.txt" ]; then
    cp "$ROOT_DIR/LICENSE-Apache-2.0.txt" "$PACKAGE_DIR/"
fi

cat > "$PACKAGE_DIR/BUILD_INFO.txt" <<EOF
name=skillctrl
version=${VERSION}
target=${TARGET_TRIPLE}
profile=${PROFILE}
binaries=$(IFS=,; printf '%s' "${BIN_NAMES[*]}")
EOF

tar -C "$WORK_DIR" -czf "$ARCHIVE_PATH" "$PACKAGE_BASENAME"

if CHECKSUM="$(write_sha256 "$ARCHIVE_PATH")"; then
    printf "%s  %s\n" "$CHECKSUM" "$(basename "$ARCHIVE_PATH")" > "$CHECKSUM_PATH"
else
    echo "warning: no sha256 tool found; checksum file was not generated" >&2
fi

echo "Package created:"
echo "  Archive: $ARCHIVE_PATH"
if [ -f "$CHECKSUM_PATH" ]; then
    echo "  Checksum: $CHECKSUM_PATH"
fi
for binary_path in "${BINARY_PATHS[@]}"; do
    echo "  Binary: $binary_path"
done
