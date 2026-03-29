#!/bin/bash
set -e

echo "Building skillctrl-desktop frontend..."
if [ ! -d "skillctrl-desktop/node_modules" ]; then
  (cd skillctrl-desktop && npm ci --no-fund --no-audit)
fi
(cd skillctrl-desktop && npm run build)

echo "Building skillctrl and skillctrl-desktop..."

# Build in release mode
cargo build --locked --release

echo "Build complete!"
echo "Binary: target/release/skillctrl"
echo "Binary: target/release/skillctrl-desktop"
echo ""
echo "To package a distributable archive, run:"
echo "  bash ./package.sh"
echo ""
echo "For the desktop app alone, the frontend assets live in skillctrl-desktop/ and are rebuilt automatically by this script."
echo ""
echo "To install globally, run:"
echo "  cargo install --locked --path crates/skillctrl-cli"
