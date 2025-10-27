# USB Port Sharing Architecture

## Problem

The original design had a critical issue: both `usb_collector.rs` and `command_executor.rs` attempted to open and use the USB serial port independently. Since a serial port can only be opened by one process at a time, this caused a "resource busy" error when `command_executor` tried to send commands while `usb_collector` had the port open.

## Solution

Implemented a centralized USB manager with message-passing architecture to share the USB port between reading and writing operations.

## Architecture

### New Module: `usb_manager.rs`

A dedicated module that owns the USB serial port connection and handles both reading and writing through a single open connection.

**Key Components:**

1. **UsbManager** - Owns the serial port and manages all I/O operations
   - Opens and maintains the USB connection
   - Handles reconnection with exponential backoff
   - Uses `tokio::select!` to multiplex reading and writing

2. **UsbCommand** - Commands sent TO the USB port
   - `SendCommand(String)` - Send a raw command string

3. **UsbMessage** - Messages FROM the USB port
   - `LineReceived(String)` - Line read from the port
   - `Connected` - Port connected successfully
   - `Disconnected` - Port connection lost

4. **UsbHandle** - Clone-able handle for sending commands
   - Wraps a channel sender
   - Provides `send_command()` method
   - Can be shared across multiple tasks

### Communication Flow

```
┌─────────────────┐
│  USB Manager    │  (owns the serial port)
│                 │
│  ┌───────────┐  │
│  │   Port    │  │
│  └─────┬─────┘  │
│        │        │
└────────┼────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
  Read      Write
    │         │
    │         │
┌───▼─────────┴───┐
│   Channels       │
│                  │
│  msg_tx  cmd_rx  │
└───┬─────────▲───┘
    │         │
    ▼         │
┌───────┐ ┌──────────┐
│ USB   │ │ Command  │
│Collec │ │ Executor │
│ tor   │ │          │
└───────┘ └──────────┘
```

### Data Flow

1. **Reading (USB → Collector)**
   - USB Manager reads lines from serial port
   - Sends `UsbMessage::LineReceived` through `msg_tx` channel
   - USB Collector receives messages from `msg_rx` channel
   - Processes lines (timestamp, filter, buffer)

2. **Writing (Command Executor → USB)**
   - Command Executor calls `usb_handle.send_command()`
   - Command sent through `cmd_tx` channel
   - USB Manager receives from `cmd_rx` channel
   - Writes to serial port immediately

### Multiplexing

The USB Manager uses `tokio::select!` to handle both operations concurrently:

```rust
loop {
    tokio::select! {
        // Handle incoming lines
        result = reader.read_line(&mut line_buffer) => {
            // Process received line
            // Send UsbMessage::LineReceived
        }
        
        // Handle outgoing commands
        Some(cmd) = self.command_rx.recv() => {
            // Write command to port
            // Flush immediately
        }
    }
}
```

## Modified Files

### 1. New File: `src/usb_manager.rs`
- **UsbManager** struct - owns serial port
- **UsbCommand** enum - command messages
- **UsbMessage** enum - notification messages  
- **UsbHandle** - clone-able command sender

### 2. Modified: `src/main.rs`
- Creates USB command and message channels
- Spawns USB Manager task
- Passes `UsbHandle` to telemetry sync
- Passes message receiver to USB collector

### 3. Modified: `src/usb_collector.rs`
- No longer opens the serial port
- Receives `UsbMessage` from channel
- Processes `LineReceived` messages
- Simplified - just message processing

### 4. Modified: `src/command_executor.rs`
- No longer opens the serial port
- Receives `UsbHandle` reference
- Calls `usb_handle.send_command()` instead of direct port access
- Removed `send_usb_command()` function

### 5. Modified: `src/telemetry_sync.rs`
- Receives `UsbHandle` in `run()` function
- Passes handle to command executor
- No other changes needed

## Benefits

1. **Single Point of Control**: Only USB Manager opens the port
2. **No Resource Conflicts**: Port is never opened twice
3. **Better Error Handling**: Connection failures handled in one place
4. **Cleaner Separation**: Reading and writing are logically separated
5. **Non-Blocking**: Commands can be sent while reading continues
6. **Scalable**: Easy to add more command senders

## Channel Sizing

- **Command Channel**: 32 messages
  - Small buffer since commands are infrequent
  - Back-pressure prevents overwhelming the USB port

- **Message Channel**: 100 messages  
  - Larger buffer for high-frequency log lines
  - Prevents blocking USB Manager on slow processing

## Reconnection Behavior

When USB connection is lost:
1. USB Manager detects error
2. Sends `UsbMessage::Disconnected`
3. Attempts reconnection with exponential backoff
4. On success, sends `UsbMessage::Connected`
5. All components continue operating (buffering, etc.)

## Testing Considerations

1. **Command Timing**: Commands are now asynchronous
   - `send_command()` returns when queued, not when written
   - For critical timing, await the result

2. **Port Availability**: Only one USB Manager per port
   - Multiple managers for same port will conflict
   - Enforce at application level (single manager task)

3. **Channel Overflow**: Monitor channel capacities
   - If command channel fills (32 messages), back-pressure occurs
   - If message channel fills (100 messages), messages may be dropped

## Migration Notes

No configuration changes needed. The refactoring is purely internal architecture.

Existing functionality preserved:
- ✅ Log collection with filtering
- ✅ Command execution from hub
- ✅ Reconnection on failure
- ✅ All command types supported

## Performance Impact

Minimal overhead:
- Single additional task (USB Manager)
- Two bounded channels (small memory footprint)
- No mutex contention (channels are lock-free)
- No copying of data (ownership transfer)

## Future Enhancements

Possible improvements:
1. Add command acknowledgment/response mechanism
2. Implement command priority queue
3. Add flow control for high-speed logging
4. Support multiple USB devices
5. Add USB Manager health monitoring
