# Deployment Guide for moonblokz-probe

This guide walks through deploying the moonblokz-probe application to a Raspberry Pi Zero.

## Prerequisites

### Hardware
- Raspberry Pi Zero (or Zero W for WiFi)
- MicroSD card (8GB+ recommended)
- USB cable for connection to MoonBlokz node
- Power supply for Raspberry Pi

### Software
- Raspberry Pi OS Lite (Bullseye or newer)
- SSH access to the Pi
- Internet connectivity on the Pi

## Step 1: Cross-Compile for ARM

On your development machine:

```bash
# Install cross-compilation tool
cargo install cross

# Build for Raspberry Pi Zero (ARMv6)
cross build --target arm-unknown-linux-gnueabihf --release

# The binary will be at:
# target/arm-unknown-linux-gnueabihf/release/moonblokz-probe
```

Alternatively, for Raspberry Pi Zero 2 (ARMv7):
```bash
cross build --target armv7-unknown-linux-gnueabihf --release
```

## Step 2: Prepare the Raspberry Pi

### Initial Setup

```bash
# SSH into your Raspberry Pi
ssh pi@raspberrypi.local

# Update system
sudo apt update
sudo apt upgrade -y

# Install required packages
sudo apt install -y usbutils

# Create application directory
mkdir -p ~/moonblokz-probe
mkdir -p ~/moonblokz-probe/deployed
```

### Configure Sudoers

The probe needs passwordless sudo for specific commands:

```bash
sudo visudo

# Add these lines at the end:
pi ALL=(ALL) NOPASSWD: /sbin/reboot
pi ALL=(ALL) NOPASSWD: /bin/mount
pi ALL=(ALL) NOPASSWD: /bin/umount
```

## Step 3: Transfer Files

From your development machine:

```bash
# Copy the binary
scp target/arm-unknown-linux-gnueabihf/release/moonblokz-probe \
    pi@raspberrypi.local:~/moonblokz-probe/

# Copy example config
scp config.toml.example \
    pi@raspberrypi.local:~/moonblokz-probe/config.toml

# Copy systemd service file
scp moonblokz-probe.service \
    pi@raspberrypi.local:~/moonblokz-probe/
```

## Step 4: Configure the Application

On the Raspberry Pi:

```bash
cd ~/moonblokz-probe

# Edit config.toml with your settings
nano config.toml
```

Update the configuration:
- `usb_port`: Check with `ls /dev/tty*` (usually `/dev/ttyACM0`)
- `server_url`: Your telemetry server endpoint
- `api_key`: Your secret API key
- `node_id`: Unique identifier for this node
- `node_firmware_url`: URL for node firmware updates
- `probe_firmware_url`: URL for probe binary updates

### Finding the USB Port

```bash
# Before connecting the node
ls /dev/tty*

# Connect the MoonBlokz node

# After connecting
ls /dev/tty*

# The new device (e.g., /dev/ttyACM0) is your USB port
```

## Step 5: Install as Systemd Service

```bash
# Make binary executable
chmod +x ~/moonblokz-probe/moonblokz-probe

# Update service file paths if needed
nano moonblokz-probe.service

# Copy service file to systemd directory
sudo cp moonblokz-probe.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable moonblokz-probe

# Start the service
sudo systemctl start moonblokz-probe

# Check status
sudo systemctl status moonblokz-probe
```

## Step 6: Verify Operation

### Check Service Status

```bash
# View service status
sudo systemctl status moonblokz-probe

# View logs
sudo journalctl -u moonblokz-probe -f

# Check recent logs
sudo journalctl -u moonblokz-probe -n 100
```

### Test USB Connection

```bash
# List USB devices
lsusb

# Check serial connection
ls -l /dev/ttyACM*

# Monitor serial output (Ctrl+C to exit)
cat /dev/ttyACM0
```

## Step 7: Configure Logging

Set log level using environment variable:

```bash
# Edit service file
sudo systemctl edit moonblokz-probe

# Add in the editor:
[Service]
Environment=RUST_LOG=info

# Reload and restart
sudo systemctl daemon-reload
sudo systemctl restart moonblokz-probe
```

Log levels:
- `error`: Only errors
- `warn`: Warnings and errors
- `info`: General information (recommended)
- `debug`: Detailed debugging
- `trace`: Very verbose

## Troubleshooting

### Service Won't Start

```bash
# Check logs for errors
sudo journalctl -u moonblokz-probe -n 50

# Try running manually
cd ~/moonblokz-probe
RUST_LOG=debug ./moonblokz-probe

# Check permissions
ls -l ~/moonblokz-probe/moonblokz-probe
```

### USB Connection Issues

```bash
# Check if device is detected
lsusb
dmesg | tail -20

# Check device permissions
ls -l /dev/ttyACM0

# Add user to dialout group if needed
sudo usermod -a -G dialout pi
# Logout and login for changes to take effect
```

### Network Issues

```bash
# Test internet connectivity
ping -c 4 8.8.8.8

# Test DNS resolution
nslookup telemetry.moonblokz.com

# Test HTTPS access
curl -I https://telemetry.moonblokz.com
```

### Can't Reboot or Mount

```bash
# Verify sudoers configuration
sudo -l

# Should show NOPASSWD for reboot, mount, umount
```

## Maintenance

### Viewing Logs

```bash
# Real-time logs
sudo journalctl -u moonblokz-probe -f

# Logs since boot
sudo journalctl -u moonblokz-probe -b

# Logs from last hour
sudo journalctl -u moonblokz-probe --since "1 hour ago"
```

### Restarting Service

```bash
sudo systemctl restart moonblokz-probe
```

### Stopping Service

```bash
sudo systemctl stop moonblokz-probe
```

### Updating Configuration

```bash
# Edit config
nano ~/moonblokz-probe/config.toml

# Restart service to apply changes
sudo systemctl restart moonblokz-probe
```

### Manual Updates

The probe can self-update when commanded by the server, but you can also update manually:

```bash
# Stop service
sudo systemctl stop moonblokz-probe

# Backup old binary
cp moonblokz-probe moonblokz-probe.backup

# Copy new binary (from dev machine)
# scp new-binary pi@raspberrypi.local:~/moonblokz-probe/moonblokz-probe

# Start service
sudo systemctl start moonblokz-probe
```

## Performance Tuning

### Reduce Log Buffer Size

If memory is constrained, modify the buffer size in the code (requires rebuild):

```rust
// In main.rs, change:
let log_buffer = Arc::new(Mutex::new(types::LogBuffer::new(10000)));
// To a smaller value:
let log_buffer = Arc::new(Mutex::new(types::LogBuffer::new(5000)));
```

### Adjust Telemetry Interval

Use the `set_update_interval` command from the server to reduce upload frequency during low-activity periods.

## Security Recommendations

1. **Change Default Password**: Change the default Pi password
   ```bash
   passwd
   ```

2. **Firewall**: Configure UFW if needed
   ```bash
   sudo apt install ufw
   sudo ufw allow ssh
   sudo ufw enable
   ```

3. **API Key**: Keep the API key secret, never commit it to version control

4. **HTTPS Only**: Ensure all URLs in config use HTTPS

5. **Regular Updates**:
   ```bash
   sudo apt update && sudo apt upgrade -y
   ```

## Monitoring

### System Resources

```bash
# CPU and memory
htop

# Disk usage
df -h

# Service resource usage
systemctl status moonblokz-probe
```

### Application Health

Monitor the logs for:
- Successful telemetry syncs
- USB connection status
- Error messages
- Update notifications

## Backup

Back up important files:

```bash
# Configuration
cp ~/moonblokz-probe/config.toml ~/config.toml.backup

# Deployed firmware versions
tar czf ~/deployed-backup.tar.gz ~/moonblokz-probe/deployed/
```

## Uninstall

To completely remove the probe:

```bash
# Stop and disable service
sudo systemctl stop moonblokz-probe
sudo systemctl disable moonblokz-probe

# Remove service file
sudo rm /etc/systemd/system/moonblokz-probe.service

# Reload systemd
sudo systemctl daemon-reload

# Remove application files
rm -rf ~/moonblokz-probe
```

