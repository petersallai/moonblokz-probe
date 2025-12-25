#!/bin/bash
#
# MoonBlokz Probe Build Script
#
# This script:
#   1. Builds the probe binary for the target architecture
#   2. Calculates CRC32 of the binary
#   3. Increments the version number
#   4. Updates versioninfo/probe/version.json
#
# Usage:
#   ./build.sh                    # Build for aarch64 (default)
#   ./build.sh arm                # Build for armv7
#   ./build.sh --no-increment     # Build without incrementing version
#

set -e

# Configuration
VERSION_FILE="versioninfo/probe/version.json"
DEFAULT_TARGET="aarch64-unknown-linux-gnu"

# Parse arguments
TARGET="$DEFAULT_TARGET"
INCREMENT_VERSION=true

while [[ $# -gt 0 ]]; do
    case $1 in
        arm|armv7)
            TARGET="arm-unknown-linux-gnueabihf"
            shift
            ;;
        aarch64|arm64)
            TARGET="aarch64-unknown-linux-gnu"
            shift
            ;;
        --no-increment)
            INCREMENT_VERSION=false
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [target] [--no-increment]"
            echo ""
            echo "Targets:"
            echo "  aarch64, arm64  - Build for 64-bit ARM (default)"
            echo "  arm, armv7      - Build for 32-bit ARM"
            echo ""
            echo "Options:"
            echo "  --no-increment  - Don't increment version number"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

BINARY_PATH="target/${TARGET}/release/moonblokz-probe"

echo "========================================"
echo "MoonBlokz Probe Build"
echo "========================================"
echo "Target: $TARGET"
echo ""

# Step 1: Build the binary
echo "[1/4] Building binary..."
cross build --release --target "$TARGET"

if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found at $BINARY_PATH"
    exit 1
fi

echo "      Built: $BINARY_PATH"

# Step 2: Calculate CRC32
echo "[2/4] Calculating CRC32..."

# Use crc32 command if available, otherwise use Python
if command -v crc32 &> /dev/null; then
    CRC32=$(crc32 "$BINARY_PATH")
elif command -v python3 &> /dev/null; then
    CRC32=$(python3 -c "
import binascii
with open('$BINARY_PATH', 'rb') as f:
    data = f.read()
print(format(binascii.crc32(data) & 0xffffffff, '08x'))
")
elif command -v python &> /dev/null; then
    CRC32=$(python -c "
import binascii
with open('$BINARY_PATH', 'rb') as f:
    data = f.read()
print(format(binascii.crc32(data) & 0xffffffff, '08x'))
")
else
    echo "ERROR: No crc32 tool or Python available"
    exit 1
fi

echo "      CRC32: $CRC32"

# Step 3: Read current version and increment
echo "[3/4] Updating version info..."

if [ ! -f "$VERSION_FILE" ]; then
    echo "      Creating new version file"
    mkdir -p "$(dirname "$VERSION_FILE")"
    CURRENT_VERSION=0
else
    # Extract current version (handles both quoted and unquoted numbers)
    CURRENT_VERSION=$(grep -o '"version"[[:space:]]*:[[:space:]]*[0-9]*' "$VERSION_FILE" | grep -o '[0-9]*$' || echo "0")
    if [ -z "$CURRENT_VERSION" ]; then
        CURRENT_VERSION=0
    fi
fi

if [ "$INCREMENT_VERSION" = true ]; then
    NEW_VERSION=$((CURRENT_VERSION + 1))
    echo "      Version: $CURRENT_VERSION -> $NEW_VERSION"
else
    NEW_VERSION=$CURRENT_VERSION
    echo "      Version: $NEW_VERSION (not incremented)"
fi

# Step 4: Write updated version.json
cat > "$VERSION_FILE" << EOF
{
  "version": $NEW_VERSION,
  "crc32": "$CRC32"
}
EOF

# Step 5: Copy binary to versioninfo/probe with versioned name
echo "[4/5] Copying binary to versioninfo/probe..."
rm -f versioninfo/probe/moonblokz_probe_*
VERSIONED_BINARY="versioninfo/probe/moonblokz_probe_${NEW_VERSION}"
cp "$BINARY_PATH" "$VERSIONED_BINARY"
echo "      Copied to: $VERSIONED_BINARY"

echo "[5/5] Done!"
echo ""
echo "========================================"
echo "Build Summary"
echo "========================================"
echo "Binary:           $BINARY_PATH"
echo "Versioned Binary: $VERSIONED_BINARY"
echo "Version:          $NEW_VERSION"
echo "CRC32:            $CRC32"
echo ""
echo "Version file updated: $VERSION_FILE"
echo ""

# Show binary size
SIZE=$(ls -lh "$BINARY_PATH" | awk '{print $5}')
echo "Binary size: $SIZE"
