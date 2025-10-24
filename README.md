# moonblokz-probe

A Rust-based telemetry probe for MoonBlokz nodes running on Raspberry Pi Zero. This service bridges RP2040-based MoonBlokz nodes with a central telemetry server.

## Features

- **USB Log Collection**: Continuously captures log data from connected MoonBlokz nodes over USB serial
- **Telemetry Sync**: Periodically uploads logs to a central telemetry server
- **Remote Command Execution**: Accepts and executes commands from the server
- **Firmware Management**: Supports both node firmware updates and probe self-updates
- **Resilient Operation**: Automatic reconnection and exponential backoff on failures
- **Configurable Filtering**: Dynamic log filtering based on content

## Requirements

- Rust 1.70+ (edition 2021)
- Raspberry Pi Zero (target platform)
- USB connection to MoonBlokz node (RP2040)
- Internet connectivity for telemetry sync

## Installation

### Build from source

```bash
cargo build --release
```

### Cross-compilation for Raspberry Pi Zero

```bash
# Install cross-compilation tools
cargo install cross

# Build for ARM (Raspberry Pi Zero)
cross build --target arm-unknown-linux-gnueabihf --release
```

## Configuration

Create a `config.toml` file based on `config.toml.example`:

```toml
# Path to the node's serial port
usb_port = "/dev/ttyACM0"

# Central telemetry server details
server_url = "https://telemetry.moonblokz.com/api/v1/data"
api_key = "YOUR_SECRET_API_KEY"

# Unique identifier for this node
node_id = "node-alpha-001"

# URLs for firmware management
node_firmware_url = "https://firmware.moonblokz.com/node"
probe_firmware_url = "https://firmware.moonblokz.com/probe"
```

### Command-line Arguments

Override configuration values using command-line flags:

```bash
moonblokz-probe --config /path/to/config.toml \
                --usb-port /dev/ttyACM0 \
                --server-url https://custom.server.com/api \
                --node-id my-node-id
```

## Usage

### Running the probe

```bash
# Using default config.toml in current directory
./moonblokz-probe

# With custom config path
./moonblokz-probe --config /etc/moonblokz/config.toml

# With logging enabled
RUST_LOG=info ./moonblokz-probe
```

### Running as a service

Create a systemd service file at `/etc/systemd/system/moonblokz-probe.service`:

```ini
[Unit]
Description=MoonBlokz Telemetry Probe
After=network.target

[Service]
Type=simple
User=pi
WorkingDirectory=/home/pi/moonblokz-probe
ExecStart=/home/pi/moonblokz-probe/moonblokz-probe --config /home/pi/moonblokz-probe/config.toml
Restart=always
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl enable moonblokz-probe
sudo systemctl start moonblokz-probe
sudo systemctl status moonblokz-probe
```

## Architecture

The application consists of several concurrent modules:

### Log Collector
- Continuously reads log lines from USB serial port
- Validates log format (expects `[LEVEL]` prefixes)
- Adds ISO-8601 timestamps
- Filters logs based on configurable filter string
- Maintains in-memory buffer (max 10,000 entries)

### Telemetry Sync
- Periodically uploads buffered logs to server (default: 60s interval)
- Sends logs via HTTP POST with authentication headers
- Processes server commands in response
- Supports dynamic update intervals

### Server Commands

The probe supports the following commands from the telemetry server:

- `set_update_interval`: Configure telemetry sync frequency
- `set_log_level`: Change node log verbosity (TRACE, DEBUG, INFO, WARN, ERROR)
- `set_filter`: Update log filtering criteria
- `run_command`: Execute raw command on node
- `update_node`: Trigger node firmware update
- `update_probe`: Trigger probe self-update
- `reboot_probe`: Reboot the Raspberry Pi

### Node Firmware Update
- Checks for new firmware on startup and via command
- Downloads and verifies firmware (CRC32 checksum)
- Manages UF2 deployment to RP2040 bootloader
- Maintains version history in `deployed/` directory

### Probe Self-Update
- Checks for new probe versions on startup and via command
- Downloads and verifies new binary
- Updates `start.sh` script
- Reboots system to apply update

## Security

- All network communications use HTTPS
- API key authentication for telemetry server
- Requires passwordless sudo for specific commands:
  - `sudo reboot`
  - `sudo mount` (for firmware updates)

Configure sudoers on Raspberry Pi:

```bash
sudo visudo
# Add line:
pi ALL=(ALL) NOPASSWD: /sbin/reboot, /bin/mount, /bin/umount
```

## Log Levels

Node log messages must include level tags:
- `[TRACE]` - Most verbose
- `[DEBUG]` - Debug information
- `[INFO]` - Informational messages
- `[WARN]` - Warnings
- `[ERROR]` - Errors

## Telemetry API

### Request Format

```http
POST /api/v1/data HTTP/1.1
Host: telemetry.moonblokz.com
X-Node-ID: node-alpha-001
X-Api-Key: YOUR_SECRET_API_KEY
Content-Type: application/json

{
  "logs": [
    {
      "timestamp": "2023-10-27T10:00:00Z",
      "message": "[INFO] Node initialized."
    },
    {
      "timestamp": "2023-10-27T10:00:05Z",
      "message": "[DEBUG] Packet received from peer."
    }
  ]
}
```

### Response Format

```json
[
  {"command": "set_log_level", "level": "DEBUG"},
  {"command": "set_filter", "value": "network"},
  {"command": "run_command", "value": "/status"}
]
```

## Development

### Project Structure

```
moonblokz-probe/
├── src/
│   ├── main.rs           # Application entry point
│   ├── config.rs         # Configuration loading
│   ├── types.rs          # Shared data structures
│   ├── log_collector.rs  # USB serial log collection
│   ├── telemetry_sync.rs # Server communication
│   ├── commands.rs       # Command execution
│   ├── node_update.rs    # Node firmware updates
│   └── probe_update.rs   # Probe self-updates
├── deployed/             # Firmware version storage
├── Cargo.toml
└── config.toml
```

### Testing

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check code
cargo clippy
cargo fmt
```

## License

See LICENSE file for details.

## Support

For issues and questions, please contact the MoonBlokz team.



moonblokz-probe is a command line application that gathers data from a MoonBlokz node (RP2040 based radio based blockchain) via USB and sends it to the central telemetry hub (HTTPS based cloud app). It can also send commands to the MoonBlokz node and update its firmware. The probe runs on a Rapsberry Pi zero.

## Command line parameters
The application requires the following command line parameters:
- the path to the USB port to connect.
- the url of the central telemetry server
- the identifier of the node
- api_key for the telemetry server
- URL for node firmware version check (NODE_FIRMWARE_URL)
- URL for app version check (PROBE_FIRMWARE_URL)

## Working model
moonblokz-probe is a Tokio based async application. 

I. It has one event loop, that do the following tasks:


I./a, Log collector from a connected nod
- Read log lines from the USB port. All lines starts with a [LOGLEVEL] tag (TRACE/DEBUG/INFO/WARN/ERROR)
- Add a timestamp to every line
- Filter the log lines. The filter logic is the following:
  - The filter logic can be changed at runtime
  - The filter value is a string and only the lines containing this string are stored.
- The filtered lines should be stored in memory.

I./b Updating the telemetry server
The moonblokz-probe periodically do a network update:
- connects to the configured url and sends the collected (and filtered) log lines using a HTTP Post request (and delete the log lines from memory after a succefull POST). Both node_identifier and api_key shall be transmitted via a header for all update requests.
- The post response can contains the following commands (in JSON format):
  - set_update_interwall with start time, end time, a period in seconds to send updates between these times and a period in seconds to send updates after end time.
  - set log level: The possible parameters are TRACE/DEBUG/INFO/WARN/ERROR/TM. The probe can set the probed system's log level by sending one of the following commands to the USB: /LT\r\n,/LD\r\n,/LI\r\n,/LW\r\n,/LE\r\n
  - set filter: set the filter to a given string
  - run command: send the given command to the usb port followed by \r\n
  - update_node: trigger node update mechanism
  - update_probe: trigger probe update mechanism
  - reboot_prone: reboot the raspberry pi zero that runs the probe

II. Update node
- During startup and when it is triggered by the update mechanism the probe check for a new firmware version on the given URL. 
  - On NODE_FIRMWARE_URL + "/version.json" there is a JSON file with the latest firmware version (single increasing number) and a CRC checksum
  - On NODE_FIRMWARE_URL + "moonblokz_<version>.uf2 there is the firmware
- If the current firmware of the node is smaller than the version in version.json the following happens:
  - The probe downloads the firmware UF2.
  - Checks the CRC checksum (if it is not correct, abort the process)
  - Send "/BS\r\n" to the USB to switch the node to bootselect mode.
  - Check for the connected USB drive on the moonblokz-probe's Raspberry Pi Zero
  - mount the drive to /mnt/rp2 (as root)
  - copy the uf2 file to /mnt/rp2 (the drive disconnects automatically)
  - Maintain a deployed folder
    - rm the previos uf2
    - move the installed uf2 to this deployed folder (we will use it to determine the actual version)

III. Update probe
- During startup and when it is triggered by the update mechanism the probe check for a new probe version on the given URL. 
  - On NODE_FIRMWARE_URL + "/version.json" there is a JSON file with the latest firmware version (single increasing number) and a CRC checksum
  - On PROBE_FIRMWARE_URL + "moonblokz_probe_<version> there is the probe binary
- If the current version of the probe is smaller than the version in version.json the following happens:
  - The probe downloads the new probe binary.
  - Checks the CRC checksum (if it is not correct, abort the process)
  - Maintain a deployed folder
    - rm the previos binary
    - move the installed binary to this deployed folder (we will use it to determine the actual version)
  - Update a start script (next to the binary) to start the latest version

