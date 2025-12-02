#!/bin/bash
set -e

# Extract version from Cargo.toml
VERSION=$(grep "^version" Cargo.toml | head -1 | awk -F '"' '{print $2}')
APP_NAME="netool"
ARCH=$(uname -m)
RELEASE_NAME="${APP_NAME}-v${VERSION}-linux-${ARCH}"

echo "Packaging ${RELEASE_NAME}..."

echo "Building netool..."
cargo build --release

# Create distribution directory
DIST_DIR="dist"
RELEASE_DIR="${DIST_DIR}/${RELEASE_NAME}"
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

echo "Copying binaries..."
cp target/release/netool-server "$RELEASE_DIR/"
cp target/release/netool-client "$RELEASE_DIR/"

echo "Copying static resources..."
if [ -d "static" ]; then
    cp -r static "$RELEASE_DIR/"
else
    echo "Warning: Static directory not found! Web mode might not work."
fi

echo "Creating archive..."
cd "$DIST_DIR"
tar -czf "${RELEASE_NAME}.tar.gz" "${RELEASE_NAME}"
cd ..

echo "Packaging complete."
echo "Output: ${DIST_DIR}/${RELEASE_NAME}.tar.gz"
ls -lh "${DIST_DIR}/${RELEASE_NAME}.tar.gz"
