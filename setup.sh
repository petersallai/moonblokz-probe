#!/bin/bash
#
# MoonBlokz Probe Setup Script
# 
# This script sets up a fresh Raspberry Pi OS 64-bit installation to run
# the MoonBlokz Probe daemon. It will:
#   - Install required dependencies
#   - Download the probe binary and configuration
#   - Create a systemd service
#   - Set up passwordless sudo for required operations
#
# Usage (run as pi user, NOT root):
#   curl -sSL https://your-server.com/setup.sh | bash
#
# Or download and run with custom options:
#   curl -sSL https://your-server.com/setup.sh -o setup.sh
#   chmod +x setup.sh
#   ./setup.sh --node-id 42 --api-key "your-key"
#

set -e

# ============================================================================
# Configuration - Override these with environment variables or command line args
# ============================================================================

# Base URL for downloading probe files
PROBE_DOWNLOAD_URL="${PROBE_DOWNLOAD_URL:-https://your-server.com/firmware/probe}"

# Default configuration values (can be overridden)
NODE_ID="${NODE_ID:-1}"
API_KEY="${API_KEY:-your-api-key-here}"
SERVER_URL="${SERVER_URL:-https://your-telemetry-hub.fermyon.app}"
NODE_FIRMWARE_URL="${NODE_FIRMWARE_URL:-https://your-server.com/firmware/node}"
PROBE_FIRMWARE_URL="${PROBE_FIRMWARE_URL:-https://your-server.com/firmware/probe}"
UPLOAD_INTERVAL="${UPLOAD_INTERVAL:-300}"
LOG_LEVEL="${LOG_LEVEL:-info}"

# Installation paths
INSTALL_DIR="/home/pi/moonblokz-probe"
SERVICE_NAME="moonblokz-probe"

# ============================================================================
# Color output helpers
# ============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# ============================================================================
# Parse command line arguments
# ============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        --node-id)
            NODE_ID="$2"
            NODE_ID_SET=1
            shift 2
            ;;
        --api-key)
            API_KEY="$2"
            shift 2
            ;;
        --server-url)
            SERVER_URL="$2"
            shift 2
            ;;
        --probe-url)
            PROBE_DOWNLOAD_URL="$2"
            shift 2
            ;;
        --node-firmware-url)
            NODE_FIRMWARE_URL="$2"
            shift 2
            ;;
        --probe-firmware-url)
            PROBE_FIRMWARE_URL="$2"
            shift 2
            ;;
        --upload-interval)
            UPLOAD_INTERVAL="$2"
            shift 2
            ;;
        --log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        -h|--help)
            echo "MoonBlokz Probe Setup Script"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --node-id ID            Set the node ID (default: 1)"
            echo "  --api-key KEY           Set the API key for hub authentication"
            echo "  --server-url URL        Set the telemetry hub URL"
            echo "  --probe-url URL         Set the probe binary download URL"
            echo "  --node-firmware-url URL Set the node firmware URL"
            echo "  --probe-firmware-url URL Set the probe firmware URL"
            echo "  --upload-interval SECS  Set upload interval in seconds (default: 300)"
            echo "  --log-level LEVEL       Set log level: error, warn, info, debug, trace"
            echo "  -h, --help              Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  NODE_ID, API_KEY, SERVER_URL, PROBE_DOWNLOAD_URL,"
            echo "  NODE_FIRMWARE_URL, PROBE_FIRMWARE_URL, UPLOAD_INTERVAL, LOG_LEVEL"
            exit 0
            ;;
        *)
            error "Unknown option: $1. Use --help for usage."
            ;;
    esac
done

# ============================================================================
# Pre-flight checks
# ============================================================================

info "Starting MoonBlokz Probe setup..."

# Check if running as pi user
if [ "$USER" != "pi" ]; then
    error "This script must be run as the 'pi' user, not as '$USER' or root"
fi

# Check if running on Raspberry Pi OS (Debian-based)
if [ ! -f /etc/debian_version ]; then
    error "This script is designed for Raspberry Pi OS (Debian-based systems)"
fi

# Check architecture
ARCH=$(uname -m)
if [ "$ARCH" != "aarch64" ]; then
    warn "Expected aarch64 architecture, found $ARCH. Binary may not be compatible."
fi

info "Running on $ARCH architecture"

# ============================================================================
# Interactive prompts for required configuration
# ============================================================================

# Prompt for node ID if not provided via command line or environment
if [ "$NODE_ID" = "1" ] && [ -z "$NODE_ID_SET" ]; then
    echo ""
    read -p "Enter Node ID (unique identifier for this probe): " INPUT_NODE_ID
    if [ -n "$INPUT_NODE_ID" ]; then
        NODE_ID="$INPUT_NODE_ID"
    else
        error "Node ID is required"
    fi
fi

# Prompt for API key if not provided via command line or environment
if [ "$API_KEY" = "your-api-key-here" ]; then
    echo ""
    read -p "Enter API Key (for hub authentication): " INPUT_API_KEY
    if [ -n "$INPUT_API_KEY" ]; then
        API_KEY="$INPUT_API_KEY"
    else
        error "API Key is required"
    fi
fi

echo ""
info "Configuration:"
info "  Node ID: $NODE_ID"
info "  API Key: ${API_KEY:0:8}..."
info "  Server:  $SERVER_URL"
echo ""

# ============================================================================
# Install dependencies
# ============================================================================

info "Updating package lists..."
sudo apt-get update -qq

info "Installing required packages..."
sudo apt-get install -y -qq curl

# ============================================================================
# Create installation directory
# ============================================================================

info "Creating installation directory: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# ============================================================================
# Download probe binary
# ============================================================================

info "Downloading probe binary from $PROBE_DOWNLOAD_URL..."

# First get the version info
if curl -sSfL "${PROBE_DOWNLOAD_URL}/version.json" -o version.json 2>/dev/null; then
    VERSION=$(cat version.json | grep -o '"version"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*: *"\([^"]*\)"/\1/')
    info "Latest version: $VERSION"
else
    warn "Could not fetch version.json, downloading latest binary directly"
    VERSION="latest"
fi

# Download the binary
BINARY_NAME="moonblokz-probe"
if ! curl -sSfL "${PROBE_DOWNLOAD_URL}/${BINARY_NAME}" -o "$BINARY_NAME"; then
    error "Failed to download probe binary from ${PROBE_DOWNLOAD_URL}/${BINARY_NAME}"
fi

chmod +x "$BINARY_NAME"
success "Downloaded probe binary"

# ============================================================================
# Create configuration file
# ============================================================================

info "Creating configuration file..."

cat > config.toml << EOF
# MoonBlokz Probe Configuration
# Generated by setup script on $(date)

# USB serial port path
usb_port = "/dev/ttyACM0"

# Telemetry hub server URL
server_url = "$SERVER_URL"

# API key for authentication with the hub
api_key = "$API_KEY"

# Unique node identifier
node_id = $NODE_ID

# Node firmware update URL (base URL without /version.json)
node_firmware_url = "$NODE_FIRMWARE_URL"

# Probe firmware update URL (base URL without /version.json)
probe_firmware_url = "$PROBE_FIRMWARE_URL"

# Upload interval in seconds
upload_interval_seconds = $UPLOAD_INTERVAL

# Maximum buffer size (number of log entries)
buffer_size = 10000

# Initial filter string (empty means no filtering)
filter_string = ""

# Log level (error, warn, info, debug, trace)
log_level = "$LOG_LEVEL"
EOF

success "Created config.toml"

# ============================================================================
# Create start script (for self-update capability)
# ============================================================================

info "Creating start script..."

cat > start.sh << 'EOF'
#!/bin/bash
# MoonBlokz Probe start script
# This script is used by the service and can be modified by the probe for self-updates

cd /home/pi/moonblokz-probe
exec ./moonblokz-probe --config config.toml
EOF

chmod +x start.sh
success "Created start.sh"

# ============================================================================
# Set up passwordless sudo for required operations
# ============================================================================

info "Setting up passwordless sudo for required operations..."

SUDOERS_FILE="/etc/sudoers.d/moonblokz-probe"

sudo tee "$SUDOERS_FILE" > /dev/null << 'EOF'
# MoonBlokz Probe - passwordless sudo for required operations
# This file allows the probe to:
#   - Mount/unmount the RP2040 bootloader for firmware updates
#   - Reboot the system for recovery and probe updates
#   - Run blkid for device detection

pi ALL=(ALL) NOPASSWD: /usr/bin/mount
pi ALL=(ALL) NOPASSWD: /usr/bin/umount
pi ALL=(ALL) NOPASSWD: /usr/sbin/reboot
pi ALL=(ALL) NOPASSWD: /usr/sbin/blkid
pi ALL=(ALL) NOPASSWD: /usr/bin/sync
EOF

# Set correct permissions (sudoers.d files must be 0440)
sudo chmod 0440 "$SUDOERS_FILE"

# Validate sudoers file syntax
if sudo visudo -cf "$SUDOERS_FILE"; then
    success "Configured passwordless sudo"
else
    sudo rm -f "$SUDOERS_FILE"
    error "Invalid sudoers configuration - removed"
fi

# ============================================================================
# Add user to dialout group for USB serial access
# ============================================================================

info "Adding pi user to dialout group for USB serial access..."
sudo usermod -a -G dialout pi
success "Added pi to dialout group"

# ============================================================================
# Create systemd service
# ============================================================================

info "Creating systemd service..."

sudo tee /etc/systemd/system/${SERVICE_NAME}.service > /dev/null << EOF
[Unit]
Description=MoonBlokz Probe - RP2040 to Telemetry Hub Bridge
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=pi
Group=pi
WorkingDirectory=${INSTALL_DIR}
ExecStart=${INSTALL_DIR}/start.sh

# Restart policy
Restart=always
RestartSec=10

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=moonblokz-probe

[Install]
WantedBy=multi-user.target
EOF

success "Created systemd service"

# ============================================================================
# Enable and start service
# ============================================================================

info "Reloading systemd daemon..."
sudo systemctl daemon-reload

info "Enabling service to start on boot..."
sudo systemctl enable ${SERVICE_NAME}

info "Starting service..."
sudo systemctl start ${SERVICE_NAME}

# Give it a moment to start
sleep 2

# Check if service is running
if sudo systemctl is-active --quiet ${SERVICE_NAME}; then
    success "Service is running"
else
    warn "Service may not have started correctly. Check logs with: journalctl -u ${SERVICE_NAME} -f"
fi

# ============================================================================
# Summary
# ============================================================================

echo ""
echo "=============================================="
echo -e "${GREEN}MoonBlokz Probe Setup Complete!${NC}"
echo "=============================================="
echo ""
echo "Installation directory: $INSTALL_DIR"
echo "Configuration file:     $INSTALL_DIR/config.toml"
echo "Service name:           $SERVICE_NAME"
echo ""
echo "Useful commands:"
echo "  View logs:        journalctl -u ${SERVICE_NAME} -f"
echo "  Restart service:  sudo systemctl restart ${SERVICE_NAME}"
echo "  Stop service:     sudo systemctl stop ${SERVICE_NAME}"
echo "  Check status:     sudo systemctl status ${SERVICE_NAME}"
echo "  Edit config:      nano $INSTALL_DIR/config.toml"
echo ""
echo -e "${YELLOW}Note: A reboot may be required for group changes to take effect.${NC}"
echo "      If USB serial access doesn't work, run: sudo reboot"
echo ""
