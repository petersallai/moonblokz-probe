# Logging Implementation

## Overview

The MoonBlokz Probe application now includes comprehensive logging via the `simple_logger` crate. This provides structured, timestamped log output to help with debugging and monitoring.

## Implementation Details

### Dependencies Added

- `log = "0.4"` - Rust's standard logging facade
- `simple_logger = "5.0"` - Simple logging implementation with timestamp support

### Log Levels

The application supports 5 log levels (from least to most verbose):

1. **ERROR** - Critical errors that need immediate attention
2. **WARN** - Warning conditions that should be reviewed
3. **INFO** - Normal operational messages (default)
4. **DEBUG** - Detailed debugging information
5. **TRACE** - Very detailed trace-level debugging

### Usage

Control log level via the `config.toml` file:

```toml
# Log level (error, warn, info, debug, trace, default: info)
log_level = "info"
```

Examples:

```toml
# Production - minimal output
log_level = "error"

# Default - normal operational messages
log_level = "info"

# Troubleshooting - detailed debugging
log_level = "debug"

# Deep debugging - very verbose
log_level = "trace"
```

After changing the log level in `config.toml`, restart the probe for changes to take effect.

### What Gets Logged

#### INFO Level (Default)
- Configuration loading
- USB connection status
- Successful telemetry uploads
- Command execution
- Firmware update status
- Version information

#### DEBUG Level
- Individual log lines received from USB
- Telemetry upload details (number of entries)
- Internal state changes

#### WARN Level
- Failed checksum verifications (with fallback)
- Unknown log levels or commands
- Upload failures with status codes

#### ERROR Level
- USB connection failures with retry information
- Telemetry upload errors
- Node firmware update failures
- Probe update failures
- Command execution errors
- Task termination (unexpected)

### Log Format

All logs include:
- Timestamp in UTC
- Log level
- Module path
- Message

Example output:
```
2025-10-27 10:30:15 [INFO] moonblokz_probe: Loaded configuration from "config.toml"
2025-10-27 10:30:15 [INFO] moonblokz_probe: Node ID: 21
2025-10-27 10:30:15 [INFO] moonblokz_probe::usb_collector: Connected to USB port: /dev/ttyACM0
2025-10-27 10:30:20 [DEBUG] moonblokz_probe::telemetry_sync: Uploading 5 log entries to hub
2025-10-27 10:30:21 [INFO] moonblokz_probe::telemetry_sync: Successfully uploaded telemetry
```

### Code Changes

All modules were updated to use logging macros instead of `println!` and `eprintln!`:

- `main.rs` - Initialization and task monitoring
- `usb_collector.rs` - USB connection and log collection
- `telemetry_sync.rs` - Telemetry upload and command processing
- `update_manager.rs` - Firmware update operations
- `command_executor.rs` - Command execution

### Systemd Integration

The systemd service file includes the log level setting. Logs are automatically captured by journald:

```bash
# View logs
sudo journalctl -u moonblokz-probe -f

# View logs at specific level
sudo journalctl -u moonblokz-probe -f -p err  # errors only
sudo journalctl -u moonblokz-probe -f -p info # info and above
```

### Benefits

1. **Structured Output** - Consistent format across all modules
2. **Timestamped** - All logs include UTC timestamps
3. **Filterable** - Easy to filter by log level in production
4. **Debugging** - Debug and trace levels help troubleshoot issues
5. **Production Ready** - Can reduce verbosity in production with ERROR level
6. **Systemd Compatible** - Works seamlessly with journald

### Performance

The `simple_logger` crate has minimal overhead:
- Log messages are formatted on-demand
- No buffering or complex processing
- Negligible impact on performance
- Can be reduced to ERROR level in production for even less overhead

## Troubleshooting

### Enable Debug Logging

If you encounter issues, edit your `config.toml`:

```toml
log_level = "debug"
```

Then restart the probe:

```bash
./target/release/moonblokz-probe
# or if running as service:
sudo systemctl restart moonblokz-probe
```

### Capture Logs to File

```bash
./target/release/moonblokz-probe 2>&1 | tee probe.log
```

### View Systemd Logs

```bash
# Follow logs in real-time
sudo journalctl -u moonblokz-probe -f

# View recent logs
sudo journalctl -u moonblokz-probe -n 100

# View logs for specific time period
sudo journalctl -u moonblokz-probe --since "1 hour ago"
```
