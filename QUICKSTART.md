# Quick Start Guide

Get moonblokz-probe up and running in minutes.

## Prerequisites

- Rust toolchain installed (https://rustup.rs/)
- MoonBlokz node connected via USB
- Access to a telemetry server

## 1. Clone and Build

```bash
git clone <repository-url>
cd moonblokz-probe
cargo build --release
```

## 2. Configure

```bash
# Copy example config
cp config.toml.example config.toml

# Edit configuration
nano config.toml
```

Minimal required settings:
```toml
usb_port = "/dev/ttyACM0"  # Your USB serial port
server_url = "https://your-telemetry-server.com/api/v1/data"
api_key = "your-secret-api-key"
node_id = "unique-node-identifier"
node_firmware_url = "https://your-firmware-server.com/node"
probe_firmware_url = "https://your-firmware-server.com/probe"
```

## 3. Find Your USB Port

### Linux/macOS
```bash
# List serial devices
ls /dev/tty*

# Or use lsusb (Linux)
lsusb
```

### Common ports
- Linux: `/dev/ttyACM0`, `/dev/ttyUSB0`
- macOS: `/dev/tty.usbmodem*`, `/dev/cu.usbserial*`

## 4. Run

```bash
# Simple run
cargo run --release

# With logging
RUST_LOG=info cargo run --release

# With custom config
cargo run --release -- --config /path/to/config.toml
```

## 5. Verify Operation

You should see output like:
```
[INFO] Starting moonblokz-probe
[INFO] Configuration loaded successfully
[INFO] Checking for probe self-update
[INFO] Probe is up to date
[INFO] Checking for node firmware update
[INFO] Node firmware is up to date
[INFO] Attempting to connect to USB port: /dev/ttyACM0
[INFO] Connected to USB port: /dev/ttyACM0
[INFO] All tasks started successfully
[INFO] Next telemetry sync in 60 seconds
```

## Command-Line Options

```bash
# Show help
cargo run --release -- --help

# Override USB port
cargo run --release -- --usb-port /dev/ttyACM1

# Override server URL
cargo run --release -- --server-url https://custom-server.com/api

# Override node ID
cargo run --release -- --node-id my-custom-node-id

# Combine options
cargo run --release -- \
  --config /etc/moonblokz/config.toml \
  --usb-port /dev/ttyACM0 \
  --node-id node-001
```

## Troubleshooting

### Can't connect to USB port

```bash
# Check if device exists
ls -l /dev/ttyACM0

# Check permissions (Linux)
sudo usermod -a -G dialout $USER
# Then logout and login

# Test with screen
screen /dev/ttyACM0 115200
# Press Ctrl+A, then K to exit
```

### Invalid configuration

```bash
# Validate TOML syntax
cargo run --release -- --config config.toml
# Will show parsing errors if config is invalid
```

### Network connection issues

```bash
# Test server connectivity
curl -I https://your-telemetry-server.com

# Check DNS resolution
nslookup your-telemetry-server.com
```

### Build errors

```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

## Next Steps

- See [DEPLOYMENT.md](DEPLOYMENT.md) for production deployment on Raspberry Pi
- See [README.md](README.md) for detailed documentation
- Configure your telemetry server to send commands to the probe

## Testing Without Hardware

To test without a physical MoonBlokz node:

1. Create a virtual serial port (Linux):
   ```bash
   socat -d -d pty,raw,echo=0 pty,raw,echo=0
   # Note the PTY devices created, e.g., /dev/pts/2 and /dev/pts/3
   ```

2. Write test data to one end:
   ```bash
   while true; do 
     echo "[INFO] Test log message at $(date)"
     sleep 1
   done > /dev/pts/3
   ```

3. Configure probe to read from the other end:
   ```toml
   usb_port = "/dev/pts/2"
   ```

4. Run the probe and verify it collects the test logs

## Development Mode

For active development:

```bash
# Auto-rebuild on changes (requires cargo-watch)
cargo install cargo-watch
cargo watch -x 'run'

# With logging
RUST_LOG=debug cargo watch -x 'run'

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt --check
```

## Production Checklist

Before deploying to production:

- [ ] Set appropriate log level (`info` or `warn`)
- [ ] Configure real telemetry server URL
- [ ] Set unique node ID
- [ ] Secure API key (don't commit to git)
- [ ] Test USB connection
- [ ] Test telemetry upload
- [ ] Configure automatic startup (systemd)
- [ ] Set up log rotation
- [ ] Configure passwordless sudo for required commands

## Getting Help

- Check logs: `RUST_LOG=debug cargo run --release`
- Review error messages carefully
- Ensure all configuration values are correct
- Test components individually (USB, network, etc.)
- See full documentation in README.md
