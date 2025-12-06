# MoonBlokz Probe - AI Coding Agent Instructions

## Project Overview
This is a Rust daemon running on Raspberry Pi Zero that bridges RP2040 radio test nodes to a cloud telemetry infrastructure. It reads logs from USB serial, buffers them, uploads to a hub, executes remote commands, and manages firmware updates for both the node and itself.

## Architecture: Message-Passing with Centralized USB Manager

**Critical Design Pattern**: USB serial port is a **single-owner resource**. Multiple components need access, so we use message-passing channels instead of shared locks.

### Core Components & Data Flow
```
main.rs spawns 5 concurrent tasks:
├─ usb_manager.rs (owns USB port, uses tokio::select! for read/write mux)
│  ├─ Sends: UsbMessage (LineReceived/Connected/Disconnected) → usb_collector
│  └─ Receives: UsbCommand (SendCommand) ← command_executor, update_manager
├─ usb_collector.rs (receives messages, filters, timestamps, buffers)
├─ telemetry_sync.rs (uploads buffer, receives commands from hub, updates upload_interval)
├─ update_manager.rs (node firmware + probe self-update)
│  ├─ perform_node_firmware_update: detects/mounts bootloader, reboots on failure
│  └─ check_and_update_probe: downloads binary, updates start.sh, reboots
└─ (command_executor called by telemetry_sync, not spawned)
```

### Shared State Pattern
Use `Arc<RwLock<T>>` for shared mutable state, NOT cloning:
- `buffer: Arc<RwLock<Vec<LogEntry>>>` - log entries awaiting upload
- `filter_string: Arc<RwLock<String>>` - dynamic log filtering
- `upload_interval: Arc<RwLock<Duration>>` - changes via `set_update_interval` command

### USB Communication Pattern
**NEVER** open the USB port directly in `usb_collector.rs`, `command_executor.rs`, or `update_manager.rs`.
- To send: Clone `UsbHandle` and call `usb_handle.send_command(String)`
- To receive: Accept `mpsc::Receiver<UsbMessage>` in function signature
- See `USB_ARCHITECTURE.md` for the "why" behind this design

## Key Commands & Workflows

### Building
```bash
cargo build --release  # For cross-compilation, target armv7-unknown-linux-gnueabihf
```

### Testing Device Detection (Node Firmware Update)
The bootloader detection in `update_manager.rs::wait_for_bootloader_device()`:
- Polls `/dev` for devices with label "RPI-RP2" (uses `blkid`)
- Mounts at `/tmp/rpi-rp2-bootloader` with `sudo mount -t vfat`
- Requires passwordless sudo for mount/umount/reboot

### Configuration
- Config loaded from `config.toml` (see `config.toml.example`)
- `log_level` controls probe's own logging (not node logs) - parsed in `main.rs`
- USB port typically `/dev/ttyACM0`

## Project-Specific Patterns

### Error Handling
- Use `anyhow::Result<T>` for functions that can fail
- Log errors with `log::error!` before returning
- **Firmware update failures trigger system reboot** - see `check_and_update_node_firmware()`

### Async/Tokio Usage
- All I/O is async (tokio::fs, tokio_serial, reqwest)
- Use `tokio::time::sleep()` not `std::thread::sleep()`
- Channel sizes: 32 for commands (infrequent), 100 for messages (high-frequency logs)

### Command Execution (`command_executor.rs`)
Hub sends JSON commands in upload response. Implementation:
- `set_update_interval`: Parses ISO 8601 timestamps, calculates active/inactive periods, updates shared `upload_interval` - see `UploadSchedule` struct
- `set_log_level`: Sends USB commands `/LT`, `/LD`, `/LI`, `/LW`, `/LE`
- `set_filter`: Updates shared `filter_string`
- `run_command`: Arbitrary USB command passthrough
- `update_node`/`update_probe`/`reboot_probe`: Trigger respective operations

### Firmware Update Workflow (Node)
1. Download & verify CRC32
2. Send `/BS` to enter bootloader
3. **Auto-detect bootloader device** - polls `/dev` with 30s timeout
4. Mount, copy firmware.uf2, sync, unmount (triggers reboot)
5. **On any failure: reboot Pi to recover clean state**

## Dependencies & Integration Points

### External Systems
- **Telemetry Hub**: HTTPS POST to `{server_url}/update` with `X-Node-ID` and `X-Api-Key` headers
- **Node Firmware URL**: `{node_firmware_url}/version.json` and `moonblokz_{version}.uf2`
- **Probe Firmware URL**: `{probe_firmware_url}/version.json` and `moonblokz_probe_{version}`

### Required System Commands (passwordless sudo)
- `sudo mount -t vfat` / `sudo umount` - bootloader mounting
- `sudo reboot` - system recovery and probe updates
- `blkid` - bootloader device detection
- `sync` - filesystem flush

## Critical Files
- `moonblokz_test_infrastructure_full_spec.md` - authoritative specification (80+ pages)
- `USB_ARCHITECTURE.md` - explains message-passing design for USB port sharing
- `src/usb_manager.rs` - the core abstraction that prevents resource conflicts
- `src/command_executor.rs` - all hub command implementations, including `set_update_interval` with time window logic

## Common Pitfalls
1. **Don't open USB port directly** - always use UsbHandle or receive UsbMessage
2. **Don't use `std::fs`** - use `tokio::fs` for async operations
3. **Function signatures**: When adding features to command_executor, telemetry_sync, or update_manager, remember to pass `UsbHandle` and shared state `Arc<RwLock<>>` refs
4. **Bootloader detection**: If extending node firmware update, remember the 30-second timeout and reboot-on-failure pattern
5. **Logging**: Use `log::info!`, `log::error!`, etc. - NOT `println!`

## Target Deployment
Raspberry Pi Zero (W/2W) running Raspberry Pi OS Lite, headless, with systemd service auto-start. Cross-compile for faster builds.
