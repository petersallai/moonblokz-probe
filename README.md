# MoonBlokz Probe

A headless Rust daemon that runs on a Raspberry Pi Zero and acts as the bridge between a MoonBlokz RP2040 node and the telemetry infrastructure.

## Features

- **Log Ingestion**: Reads log lines from the node's USB serial console, timestamps them, and filters them according to a dynamic filter string
- **Buffering**: Maintains an in-memory queue of recent log entries with configurable buffer size
- **Periodic Upload**: Assembles buffered logs into batches and sends them to the telemetry hub via HTTPS POST
- **Command Execution**: Executes commands received from the hub (change log level, update filter, firmware updates, etc.)
- **Firmware Management**: Automatically checks for and applies firmware updates for both the RP2040 node and the probe itself

## Requirements

- Rust 1.70 or later
- Raspberry Pi Zero (or compatible device)
- RP2040-based MoonBlokz node connected via USB

## Raspberry Pi Zero Setup

This section provides the probe-specific setup steps for a Raspberry Pi Zero running Raspberry Pi OS Lite. It assumes you already have the OS installed and SSH access configured.

### Prerequisites

- Raspberry Pi Zero W or Zero 2 W with Raspberry Pi OS Lite installed
- SSH access to the Pi
- RP2040 node connected via USB
- Internet connectivity configured

### System Configuration

#### 1. Configure USB Serial Access

Grant the user permission to access USB serial devices:

```bash
sudo usermod -a -G dialout $USER
```

**Important**: Log out and log back in for group changes to take effect:
```bash
exit
# SSH back in
ssh pi@<raspberry-pi-ip>
```

Verify the RP2040 node is detected:
```bash
ls -l /dev/ttyACM*
```

You should see output like:
```
crw-rw---- 1 root dialout 166, 0 Oct 27 10:30 /dev/ttyACM0
```

#### 2. Configure Passwordless Sudo for Required Operations

The probe needs passwordless sudo for:
- Rebooting the system (`reboot`)
- Mounting/unmounting filesystems (for node firmware updates)

Edit the sudoers file:
```bash
sudo visudo
```

Add the following lines at the end:
```
# Allow moonblokz-probe to reboot without password
pi ALL=(ALL) NOPASSWD: /sbin/reboot

# Allow moonblokz-probe to mount/umount for firmware updates
pi ALL=(ALL) NOPASSWD: /bin/mount
pi ALL=(ALL) NOPASSWD: /bin/umount
```

Save and exit (Ctrl+X, then Y, then Enter in nano).

Test passwordless reboot:
```bash
sudo reboot
```

#### 3. Create Working Directory Structure

```bash
mkdir -p ~/moonblokz-probe/deployed
cd ~/moonblokz-probe
```

### Installing the Probe

#### 1. Transfer the Pre-Compiled Binary

From your development machine, copy the cross-compiled binary to the Raspberry Pi:

```bash
# On your development machine
scp target/release/moonblokz-probe pi@<raspberry-pi-ip>:~/moonblokz-probe/
```

Or if cross-compiled for ARM:
```bash
scp target/armv7-unknown-linux-gnueabihf/release/moonblokz-probe pi@<raspberry-pi-ip>:~/moonblokz-probe/
```

Make the binary executable:
```bash
chmod +x ~/moonblokz-probe/moonblokz-probe
```

#### 2. Create Configuration File

```bash
cp config.toml.example config.toml
nano config.toml
```

Edit the configuration according to your setup:

```toml
# USB serial port for the RP2040 node
usb_port = "/dev/ttyACM0"

# Telemetry hub URL
server_url = "https://your-telemetry-hub.example.com"

# API key for authentication
api_key = "your-secret-api-key-here"

# Unique node identifier
node_id = 1001

# Firmware update URLs
node_firmware_url = "https://firmware.example.com/node"
probe_firmware_url = "https://firmware.example.com/probe"

# Upload interval in seconds
upload_interval_seconds = 300

# Maximum buffer size
buffer_size = 10000

# Log filter (empty = no filtering)
filter_string = ""

# Log level (error, warn, info, debug, trace)
log_level = "info"
```

Save and exit (Ctrl+X, Y, Enter).

#### 3. Test the Probe

Run the probe manually to verify it works:

```bash
./moonblokz-probe --config config.toml
```

You should see log output indicating:
- USB connection established
- Configuration loaded
- Telemetry sync starting
- Update managers initialized

Press Ctrl+C to stop.

### Configuring Automatic Startup

#### 1. Create Systemd Service File

```bash
sudo nano /etc/systemd/system/moonblokz-probe.service
```

Add the following content:

```ini
[Unit]
Description=MoonBlokz Probe - RP2040 Telemetry Bridge
Documentation=https://github.com/yourusername/moonblokz-probe
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=pi
Group=pi
WorkingDirectory=/home/pi/moonblokz-probe
ExecStart=/home/pi/moonblokz-probe/moonblokz-probe --config /home/pi/moonblokz-probe/config.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
PrivateTmp=yes
NoNewPrivileges=yes

[Install]
WantedBy=multi-user.target
```

Save and exit.

#### 2. Enable and Start the Service

```bash
# Reload systemd to recognize the new service
sudo systemctl daemon-reload

# Enable the service to start on boot
sudo systemctl enable moonblokz-probe

# Start the service now
sudo systemctl start moonblokz-probe

# Check the status
sudo systemctl status moonblokz-probe
```

You should see output indicating the service is "active (running)".

#### 3. View Logs

Monitor the probe's operation:

```bash
# View live logs
sudo journalctl -u moonblokz-probe -f

# View recent logs
sudo journalctl -u moonblokz-probe -n 100

# View logs since last boot
sudo journalctl -u moonblokz-probe -b
```

## Configuration

1. Copy the example configuration file:
   ```bash
   cp config.toml.example config.toml
   ```

2. Edit `config.toml` with your settings:
   - `usb_port`: Path to the USB serial port (e.g., `/dev/ttyACM0`)
   - `server_url`: URL of your telemetry hub
   - `api_key`: Shared secret for authentication
   - `node_id`: Unique identifier for this node
   - `node_firmware_url`: Base URL for node firmware updates
   - `probe_firmware_url`: Base URL for probe firmware updates
   - `upload_interval_seconds`: Interval between telemetry uploads (default: 300)
   - `buffer_size`: Maximum number of log entries to hold in memory (default: 10,000)
   - `filter_string`: Initial substring filter for logs (empty = no filtering)
   - `log_level`: Log level for probe application logging - error, warn, info, debug, trace (default: info)

## Building

```bash
cargo build --release
```

The binary will be created at `target/release/moonblokz-probe`.

## Running

```bash
./target/release/moonblokz-probe --config config.toml
```

Or use the default config location:

```bash
./target/release/moonblokz-probe
```

### Log Levels

You can control the verbosity of the probe's own logging (not the node logs) in the `config.toml` file:

```toml
# Log level (error, warn, info, debug, trace, default: info)
log_level = "debug"
```

Available log levels (from least to most verbose):
- `error` - Only errors
- `warn` - Warnings and errors
- `info` - Informational messages, warnings, and errors (default)
- `debug` - Debug information plus all above
- `trace` - Trace-level details plus all above

## Running as a Service

For automatic startup on boot, create a systemd service:

1. Create `/etc/systemd/system/moonblokz-probe.service`:

```ini
[Unit]
Description=MoonBlokz Probe
After=network.target

[Service]
Type=simple
User=pi
WorkingDirectory=/home/pi/moonblokz-probe
ExecStart=/home/pi/moonblokz-probe/target/release/moonblokz-probe --config /home/pi/moonblokz-probe/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

2. Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable moonblokz-probe
sudo systemctl start moonblokz-probe
```

3. Check status:

```bash
sudo systemctl status moonblokz-probe
```

## Supported Commands

The probe can execute the following commands received from the telemetry hub:

- `set_update_interval`: Modify the probe's upload schedule
- `set_log_level`: Change verbosity on the RP2040 node (TRACE, DEBUG, INFO, WARN, ERROR)
- `set_filter`: Update the in-memory substring filter
- `run_command`: Execute an arbitrary USB command on the node
- `update_node`: Trigger node firmware update
- `update_probe`: Trigger probe self-update
- `reboot_probe`: Reboot the Raspberry Pi

## Firmware Updates

### Node Firmware

The probe periodically checks for node firmware updates at `{node_firmware_url}/version.json`. When a new version is detected, it:

1. Downloads the UF2 file
2. Verifies the CRC32 checksum
3. Enters bootloader mode on the RP2040
4. Copies the firmware to the bootloader
5. Records the new version in the `deployed/` directory

### Probe Self-Update

The probe periodically checks for its own updates at `{probe_firmware_url}/version.json`. When a new version is detected, it:

1. Downloads the new binary
2. Verifies the checksum
3. Replaces the old binary in `deployed/`
4. Updates the `start.sh` script
5. Reboots the system

## Permissions

For firmware updates and reboots to work, the probe needs passwordless sudo access for:

- `reboot`
- Mounting filesystems (for node firmware updates)

Add to `/etc/sudoers` (using `visudo`):

```
pi ALL=(ALL) NOPASSWD: /sbin/reboot
pi ALL=(ALL) NOPASSWD: /bin/mount
pi ALL=(ALL) NOPASSWD: /bin/umount
```

## Troubleshooting

- **USB connection issues**: Check that the serial port path is correct and the user has permission to access it
- **Upload failures**: Verify the server URL and API key are correct
- **Firmware updates**: Ensure the firmware URLs are accessible and version.json files are properly formatted

## Architecture

The probe uses Tokio for async I/O and runs four concurrent tasks:

1. **USB Log Collector**: Reads from serial port and buffers logs
2. **Telemetry Sync**: Periodically uploads logs and retrieves commands
3. **Node Update Manager**: Checks for and applies node firmware updates
4. **Probe Update Manager**: Checks for and applies probe self-updates

All tasks communicate through shared Arc<RwLock<>> data structures for thread-safe access.

## License

See LICENSE file for details.
