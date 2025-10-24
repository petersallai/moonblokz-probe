#!/bin/bash
# Test script for moonblokz-probe
# This script helps verify the setup before running the probe

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "moonblokz-probe Setup Verification"
echo "========================================="
echo ""

# Check if config file exists
echo -n "Checking config.toml... "
if [ -f "config.toml" ]; then
    echo -e "${GREEN}✓ Found${NC}"
else
    echo -e "${RED}✗ Missing${NC}"
    echo "  Run: cp config.toml.example config.toml"
    exit 1
fi

# Check if binary exists
echo -n "Checking binary... "
if [ -f "target/release/moonblokz-probe" ]; then
    echo -e "${GREEN}✓ Found${NC}"
elif [ -f "moonblokz-probe" ]; then
    echo -e "${GREEN}✓ Found${NC}"
else
    echo -e "${RED}✗ Missing${NC}"
    echo "  Run: cargo build --release"
    exit 1
fi

# Check USB devices
echo ""
echo "Available serial devices:"
if command -v lsusb &> /dev/null; then
    lsusb
fi
echo ""
ls -la /dev/tty* 2>/dev/null | grep -E "(ACM|USB)" || echo "  No USB serial devices found"

# Check config values
echo ""
echo "Configuration values:"
if [ -f "config.toml" ]; then
    echo "  USB Port: $(grep usb_port config.toml | cut -d'"' -f2)"
    echo "  Server URL: $(grep server_url config.toml | cut -d'"' -f2)"
    echo "  Node ID: $(grep node_id config.toml | cut -d'"' -f2)"
fi

# Check network connectivity
echo ""
echo -n "Checking internet connectivity... "
if ping -c 1 8.8.8.8 &> /dev/null; then
    echo -e "${GREEN}✓ Connected${NC}"
else
    echo -e "${YELLOW}⚠ No internet connection${NC}"
fi

# Check if deployed directory exists
echo -n "Checking deployed/ directory... "
if [ -d "deployed" ]; then
    echo -e "${GREEN}✓ Exists${NC}"
else
    echo -e "${YELLOW}⚠ Creating${NC}"
    mkdir -p deployed
fi

# Check permissions for serial port
echo ""
USB_PORT=$(grep usb_port config.toml | cut -d'"' -f2)
if [ -e "$USB_PORT" ]; then
    echo -n "Checking permissions for $USB_PORT... "
    if [ -r "$USB_PORT" ] && [ -w "$USB_PORT" ]; then
        echo -e "${GREEN}✓ OK${NC}"
    else
        echo -e "${RED}✗ No access${NC}"
        echo "  Run: sudo usermod -a -G dialout \$USER"
        echo "  Then logout and login again"
    fi
else
    echo -e "${YELLOW}⚠ USB port $USB_PORT not found${NC}"
    echo "  Check if device is connected"
fi

# Check if sudo is configured for reboot
echo ""
echo -n "Checking sudo permissions... "
if sudo -n reboot --help &> /dev/null; then
    echo -e "${GREEN}✓ Passwordless sudo configured${NC}"
else
    echo -e "${YELLOW}⚠ May need password for sudo${NC}"
    echo "  For production, configure: sudo visudo"
fi

echo ""
echo "========================================="
echo "Setup verification complete!"
echo "========================================="
echo ""
echo "To run the probe:"
echo "  RUST_LOG=info cargo run --release"
echo ""
echo "Or if binary is built:"
echo "  RUST_LOG=info ./target/release/moonblokz-probe"
echo ""
