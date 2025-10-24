# Technical Specification: **moonblokz-probe**
*Converted from the original specification document.* fileciteturn0file0

**Target platform:** Raspberry Pi Zero  
**Primary language & runtime:** Rust with the Tokio async framework

---

## Table of Contents
1. [Overview](#1-overview)
2. [System Architecture](#2-system-architecture)
3. [Configuration](#3-configuration)
4. [Core Logic & Modules](#4-core-logic--modules)  
   4.1. [Main Application Loop](#41-main-application-loop)  
   4.2. [Module: Log Collector](#42-module-log-collector)  
   4.3. [Module: Telemetry Sync](#43-module-telemetry-sync)  
   4.4. [Module: Node Firmware Update](#44-module-node-firmware-update)  
   4.5. [Module: Probe Self-Update](#45-module-probe-self-update)
5. [Error Handling & Resilience](#5-error-handling--resilience)
6. [Security & Permissions](#6-security--permissions)

---

## 1. Overview
**moonblokz-probe** is a command-line background service that bridges a MoonBlokz node (RP2040-based) and a central telemetry server. It:
- Captures log data from the node over USB.
- Periodically syncs logs to the telemetry server.
- Accepts remote commands to manage both the node and the probe (including firmware updates).

The application is built in **Rust** using **Tokio** for concurrency. fileciteturn0file0

---

## 2. System Architecture
The probe runs as a stateful service with several concurrent async tasks:

- **USB Log Collector** — Continuously reads and processes log lines from the connected MoonBlokz node.  
- **Telemetry Sync Loop** — Periodically uploads buffered logs to the telemetry server and executes any commands in the response.  
- **Update Manager (Node)** — Orchestrates node firmware updates.  
- **Update Manager (Probe)** — Orchestrates self-updates of the probe binary. fileciteturn0file0

---

## 3. Configuration
Configuration is read from a single `config.toml` file; command-line flags override matching settings.

### Command-line Arguments
- `--config <path>` — Path to config (default: `./config.toml`).
- `--usb-port <path>` — Override USB serial path (e.g., `/dev/ttyACM0`).
- `--server-url <url>` — Override telemetry server URL.
- `--node-id <id>` — Override unique node identifier. fileciteturn0file0

### Example `config.toml`
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
fileciteturn0file0

---

## 4. Core Logic & Modules

### 4.1. Main Application Loop
On startup, the application initializes all modules and starts the Tokio runtime. The core tasks (Log Collector and Telemetry Sync) run concurrently. fileciteturn0file0

### 4.2. Module: Log Collector
**Responsibilities:** Interface with the MoonBlokz node via USB serial.

- **Connection:** Continuously attempts to connect to `usb_port`. Uses exponential backoff on connection failure or loss.  
- **Log ingestion:** Reads input line-by-line. Each valid line begins with a level tag: `[TRACE]`, `[DEBUG]`, `[INFO]`, `[WARN]`, `[ERROR]`. The probe attaches a UTC **ISO‑8601** timestamp (e.g., `2023-10-27T10:00:00Z`).  
- **In-memory buffer & filtering:**  
  - Maintains a runtime-configurable `filter_string`.  
  - A line is buffered only if it contains `filter_string`; if empty, all lines are stored.  
  - Buffer is a thread-safe queue (e.g., `Arc<Mutex<Vec<LogEntry>>>`).  
  - Buffer is capped at a configurable limit (e.g., 10,000 lines); oldest entries are discarded when full.

**Data structure:**
```rust
struct LogEntry {
    timestamp: String, // ISO 8601 format
    message: String,   // Original log content, including the [LEVEL] tag
}
```
fileciteturn0file0

### 4.3. Module: Telemetry Sync
Periodically communicates with the central telemetry server.

- **Update schedule:**  
  - Default interval: **60 seconds**.  
  - The interval can be dynamically adjusted by a `set_update_interval` command from the server (supports active/inactive periods within a specific time window).

- **HTTP POST request:**  
  - **URL:** `server_url`  
  - **Headers:**  
    - `X-Node-ID: <node_id>`  
    - `X-Api-Key: <api_key>`  
    - `Content-Type: application/json`  
  - **Payload example:**
    ```json
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
  - **Result handling:** On **200 OK**, the sent logs are cleared; on failure, logs remain for retry. fileciteturn0file0

- **Processing server commands:** The `200 OK` response body may contain a JSON array of commands to execute sequentially.
  - **Response example:**
    ```json
    [
      {"command": "set_log_level", "level": "DEBUG"},
      {"command": "set_filter", "value": "network"},
      {"command": "run_command", "value": "/status"}
    ]
    ```

- **Supported commands:**
  - `set_update_interval` — Sets telemetry update frequency for a time window.  
    **Payload:**  
    ```json
    {
      "start_time": "YYYY-MM-DDTHH:MM:SSZ",
      "end_time": "YYYY-MM-DDTHH:MM:SSZ",
      "active_period": 30,
      "inactive_period": 300
    }
    ```
  - `set_log_level` — Changes log verbosity on the node.  
    **Payload:** `{ "level": "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR" }`  
    **Action:** Sends the corresponding command to the USB port (e.g., `/LT\r\n`, `/LD\r\n`, …).
    <br>Example mapping:
    | Level | USB Command |
    |------:|:------------|
    | `TRACE` | `/LT\r\n` |
    | `DEBUG` | `/LD\r\n` |
    | `INFO`  | `/LI\r\n` |
    | `WARN`  | `/LW\r\n` |
    | `ERROR` | `/LE\r\n` |
  - `set_filter` — Updates in-memory log filter.  
    **Payload:** `{ "value": "some_string_to_filter_by" }`
  - `run_command` — Executes a raw command on the node.  
    **Payload:** `{ "value": "/some_command" }`  
    **Action:** Sends `value + "\r\n"` to the USB port.
  - `update_node` — Initiates the **node** firmware update process.  
  - `update_probe` — Initiates the **probe** self-update process.  
  - `reboot_probe` — Reboots the Raspberry Pi Zero via `sudo reboot` (requires passwordless sudo). fileciteturn0file0

### 4.4. Module: Node Firmware Update
Triggered on startup and by the `update_node` command.

1. **Check for latest version:** Fetch `NODE_FIRMWARE_URL/version.json` (e.g., `{ "version": 105, "crc32": "a1b2c3d4" }`).  
2. **Determine current version:** Read from the UF2 filename in `deployed/` (e.g., `moonblokz_104.uf2` → version `104`; if none, assume `0`).  
3. **Compare versions:** If `latest_version > current_version`, continue.  
4. **Download firmware:** `NODE_FIRMWARE_URL/moonblokz_<version>.uf2` to a temp path (e.g., `/tmp/firmware.uf2`).  
5. **Verify checksum:** Compute **CRC32**; abort on mismatch.  
6. **Switch node to bootloader:** Send `/BS\r\n` over USB; then disconnect serial.  
7. **Mount UF2 drive:** Poll for RP2040 bootloader device; mount (e.g., `sudo mount /dev/sdX /mnt/rp2`).  
8. **Copy firmware:** Copy UF2 to `/mnt/rp2`; device auto-unmounts and reboots.  
9. **Cleanup:** Delete old UF2 from `deployed/`; move new UF2 into `deployed/` and rename with version; log success. fileciteturn0file0

### 4.5. Module: Probe Self-Update
Triggered on startup and by the `update_probe` command.

1. **Check for latest version:** Fetch `PROBE_FIRMWARE_URL/version.json`.  
2. **Determine current version:** From the executable filename in `deployed/` (use the running binary path to locate).  
3. **Compare versions:** If `latest_version > current_version`, continue.  
4. **Download new binary:** `PROBE_FIRMWARE_URL/moonblokz_probe_<version>`; `chmod +x`.  
5. **Verify checksum:** Abort on mismatch.  
6. **Deploy new binary:** Remove prior binary from `deployed/`; move the new one into `deployed/`.  
7. **Update start script:** Atomically write `start.sh` to point to the new executable:
   ```bash
   #!/bin/bash
   # This script is auto-generated. DO NOT EDIT.
   /path/to/deployed/moonblokz_probe_<new_version> --config /path/to/config.toml
   ```
8. **Reboot:** Execute `sudo reboot` to cleanly start the new version on boot. fileciteturn0file0

---

## 5. Error Handling & Resilience
- **USB disconnection:** Continuously retry connection.  
- **Network failure:** Retry telemetry updates; buffer logs in memory (up to the cap).  
- **Invalid server response:** Log malformed JSON or unexpected statuses; retry at next interval.  
- **Update failures:** Log and abort on any failure (download, checksum, copy); retry on next trigger. fileciteturn0file0

---

## 6. Security & Permissions
- All communications with telemetry and firmware servers must use **HTTPS**.  
- Treat `api_key` as a secret.  
- Requires elevated privileges for `sudo reboot` and mounting drives (e.g., `sudo mount`). Configure a **sudoers** rule on the Pi to allow these specific commands without a password. fileciteturn0file0
