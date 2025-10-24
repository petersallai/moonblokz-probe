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
