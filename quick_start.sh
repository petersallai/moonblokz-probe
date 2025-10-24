#!/bin/bash
# Quick start script for MoonBlokz Probe

set -e

echo "MoonBlokz Probe - Quick Start"
echo "=============================="
echo ""

# Check if config exists
if [ ! -f "config.toml" ]; then
    echo "Creating config.toml from example..."
    cp config.toml.example config.toml
    echo "⚠️  Please edit config.toml with your settings before running!"
    echo ""
    exit 1
fi

# Build the project
echo "Building probe application..."
cargo build --release

# Create deployed directory if it doesn't exist
mkdir -p deployed

echo ""
echo "✅ Build complete!"
echo ""
echo "To run the probe:"
echo "  ./target/release/moonblokz-probe --config config.toml"
echo ""
echo "To install as a systemd service, see README.md"
echo ""
