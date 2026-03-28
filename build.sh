#!/bin/bash
set -e

echo "Building skillctrl..."

# Build in release mode
cargo build --locked --release

echo "Build complete!"
echo "Binary: target/release/skillctrl"
echo ""
echo "To package a distributable archive, run:"
echo "  bash ./package.sh"
echo ""
echo "To install globally, run:"
echo "  cargo install --locked --path crates/skillctrl-cli"
