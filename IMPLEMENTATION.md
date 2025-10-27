# MoonBlokz Probe - Implementation Summary

## Overview

The MoonBlokz Probe application has been successfully created according to the specification in `moonblokz_test_infrastructure_full_spec.md`. This is a headless Rust daemon that runs on a Raspberry Pi Zero and bridges the MoonBlokz RP2040 node with the telemetry infrastructure.

## Project Structure

```
moonblokz-probe/
├── Cargo.toml                          # Project dependencies and metadata
├── README.md                            # Comprehensive documentation
├── config.toml.example                  # Example configuration file
├── src/
│   ├── main.rs                         # Entry point, spawns async tasks
│   ├── config.rs                       # Configuration loading from TOML
│   ├── log_entry.rs                    # LogEntry data structure
│   ├── error.rs                        # Custom error types
│   ├── usb_collector.rs                # USB serial port log collection
│   ├── telemetry_sync.rs               # Periodic log upload to hub
│   ├── update_manager.rs               # Firmware update management
│   └── command_executor.rs             # Command execution from hub
└── target/release/moonblokz-probe      # Compiled binary
```

## Key Features Implemented

### 1. Configuration Management (`config.rs`)
- Loads configuration from TOML file
- Supports all required fields with sensible defaults
- Configurable via `--config` command-line option

### 2. USB Log Collection (`usb_collector.rs`)
- Connects to RP2040 via USB serial port
- Reads log lines continuously
- Adds ISO 8601 UTC timestamps to each line
- Applies dynamic substring filtering
- Maintains in-memory buffer with size limits (oldest entries dropped)
- Automatic reconnection with exponential backoff

### 3. Telemetry Upload (`telemetry_sync.rs`)
- Periodic uploads at configurable intervals
- Sends logs to hub's `/update` endpoint
- Includes required headers: `X-Node-ID`, `X-Api-Key`
- Handles responses and error conditions
- Retains logs on failure for retry
- Clears buffer on successful upload
- Parses and executes commands from hub response

### 4. Command Execution (`command_executor.rs`)
Supports all specified commands:
- `set_update_interval` - Modify upload schedule
- `set_log_level` - Change node verbosity (TRACE/DEBUG/INFO/WARN/ERROR)
- `set_filter` - Update substring filter
- `run_command` - Execute arbitrary USB command
- `update_node` - Trigger node firmware update
- `update_probe` - Trigger probe self-update
- `reboot_probe` - Reboot Raspberry Pi

### 5. Firmware Management (`update_manager.rs`)

#### Node Firmware Updates:
- Fetches version.json from firmware URL
- Compares with current deployed version
- Downloads UF2 files
- Verifies CRC32 checksums
- Enters bootloader mode (`/BS` command)
- Copies firmware to bootloader
- Manages deployed versions

#### Probe Self-Updates:
- Fetches version.json for probe binary
- Downloads new binary
- Verifies checksum
- Replaces binary in deployed/ directory
- Updates start.sh script
- Reboots system

### 6. Concurrent Architecture
Uses Tokio async runtime with four concurrent tasks:
1. USB log collector task
2. Telemetry sync task
3. Node firmware update manager
4. Probe self-update manager

All tasks share state via `Arc<RwLock<>>` for thread-safe access without mutexes.

## Dependencies

- **tokio** - Async runtime with full features
- **serde/serde_json** - Serialization/deserialization
- **toml** - Configuration file parsing
- **anyhow/thiserror** - Error handling
- **chrono** - Timestamp generation
- **reqwest** - HTTPS client with rustls
- **tokio-serial** - Async serial port communication
- **crc32fast** - Checksum verification
- **clap** - Command-line argument parsing

## Configuration

Example `config.toml`:
```toml
usb_port = "/dev/ttyACM0"
server_url = "https://your-telemetry-hub.fermyon.app"
api_key = "your-api-key-here"
node_id = 21
node_firmware_url = "https://example.com/firmware/node"
probe_firmware_url = "https://example.com/firmware/probe"
upload_interval_seconds = 300
buffer_size = 10000
filter_string = ""
log_level = "info"
```

The `log_level` setting controls the verbosity of the probe's own logging (not the node logs). Available levels: error, warn, info, debug, trace.

## Building and Running

```bash
# Build release version
cargo build --release

# Run with default config
./target/release/moonblokz-probe

# Run with custom config
./target/release/moonblokz-probe --config /path/to/config.toml
```

To change the log level, edit the `log_level` setting in `config.toml` and restart the probe.

## Deployment as Service

A systemd service configuration is documented in README.md for automatic startup on boot.

## Security Considerations

- All HTTP communication uses HTTPS with TLS verification
- API keys are required for all hub communications
- API keys are never logged
- Requires passwordless sudo for:
  - `reboot` command
  - Filesystem mounting (for firmware updates)

## Error Handling

- USB disconnections: Exponential backoff reconnection
- Network failures: Log retention and retry
- Malformed responses: Logged but non-fatal
- All errors logged to stderr
- Graceful degradation on subsystem failures

## Testing Recommendations

1. **USB Connection**: Test with actual RP2040 node or mock serial device
2. **Telemetry Hub**: Ensure hub is running and accessible
3. **Firmware Updates**: Test version.json format and download process
4. **Commands**: Test each command type through hub
5. **Resilience**: Test reconnection and retry logic

## Future Enhancements

The current implementation provides all core functionality specified. Potential improvements:

1. Dynamic upload interval scheduling with time windows
2. Persistent state across reboots (last_id, etc.)
3. More sophisticated bootloader detection and mounting
4. Metrics and monitoring integration
5. Configuration hot-reload
6. Better signal handling for graceful shutdown

## Compliance with Specification

The implementation follows the specification with these notes:

✅ All required features implemented
✅ Idiomatic Rust patterns used
✅ Async/await with Tokio (no mutexes, uses RwLock)
✅ Minimal cloning (uses Arc and references)
✅ Proper error handling and logging
✅ ISO 8601 UTC timestamps
✅ Command execution support
✅ Firmware update logic
✅ Exponential backoff for retries
✅ Buffer management with size limits

## Build Status

✅ Successfully compiled in release mode
✅ No compilation errors
✅ Only minor warnings for unused error variants (intentional for future use)
✅ All dependencies resolved correctly

The probe application is ready for deployment and testing!
