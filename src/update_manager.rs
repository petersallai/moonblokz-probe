use crate::config::Config;
use crate::usb_manager::UsbHandle;
use anyhow::Result;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

const CHECK_INTERVAL_SECONDS: u64 = 3600; // Check every hour
const DEPLOYED_DIR: &str = "deployed";

#[derive(Debug, Deserialize)]
struct VersionInfo {
    version: u32,
    crc32: String,
}

pub async fn run_node_update(config: Arc<Config>, usb_handle: UsbHandle) -> Result<()> {
    // Check on startup
    if let Err(e) = check_and_update_node_firmware(&config, &usb_handle).await {
        error!("Node firmware update check failed: {}", e);
    }

    loop {
        sleep(Duration::from_secs(CHECK_INTERVAL_SECONDS)).await;

        if let Err(e) = check_and_update_node_firmware(&config, &usb_handle).await {
            error!("Node firmware update check failed: {}", e);
        }
    }
}

pub async fn run_probe_update(config: Arc<Config>) -> Result<()> {
    // Check on startup
    if let Err(e) = check_and_update_probe(&config).await {
        error!("Probe update check failed: {}", e);
    }

    loop {
        sleep(Duration::from_secs(CHECK_INTERVAL_SECONDS)).await;

        if let Err(e) = check_and_update_probe(&config).await {
            error!("Probe update check failed: {}", e);
        }
    }
}

async fn check_and_update_node_firmware(config: &Config, usb_handle: &UsbHandle) -> Result<()> {
    // Fetch version info
    let version_url = format!("{}/version.json", config.node_firmware_url);
    let response = reqwest::get(&version_url).await?;
    let version_info: VersionInfo = response.json().await?;

    // Determine current version
    let current_version = get_current_node_version().await?;

    info!("Node firmware - Current: {}, Latest: {}", current_version, version_info.version);

    if version_info.version <= current_version {
        return Ok(());
    }

    info!("Updating node firmware to version {}...", version_info.version);

    // Wrap the update process to handle failures with reboot
    if let Err(e) = perform_node_firmware_update(config, usb_handle, &version_info).await {
        error!("Node firmware update failed: {}. Rebooting system to recover...", e);
        sleep(Duration::from_secs(2)).await;
        let _ = reboot_system().await;
        return Err(e);
    }

    Ok(())
}

async fn perform_node_firmware_update(config: &Config, usb_handle: &UsbHandle, version_info: &VersionInfo) -> Result<()> {
    // Download new firmware
    let firmware_url = format!("{}/moonblokz_{}.uf2", config.node_firmware_url, version_info.version);
    let response = reqwest::get(&firmware_url).await?;
    let firmware_data = response.bytes().await?;

    // Verify CRC32
    let computed_crc = crc32fast::hash(&firmware_data);
    let expected_crc =
        u32::from_str_radix(&version_info.crc32, 16).map_err(|_| anyhow::anyhow!("Invalid CRC32 format in version.json: {}", version_info.crc32))?;

    if computed_crc != expected_crc {
        return Err(anyhow::anyhow!("CRC32 mismatch: expected {:x}, got {:x}", expected_crc, computed_crc));
    }

    // Save to temporary file
    let temp_file = format!("/tmp/moonblokz_{}.uf2", version_info.version);
    fs::write(&temp_file, &firmware_data).await?;

    // Enter bootloader mode
    info!("Entering bootloader mode...");
    usb_handle.send_command("/BS".to_string()).await?;

    // Wait for bootloader device to appear and detect it
    info!("Waiting for bootloader device to appear...");
    let bootloader_device = wait_for_bootloader_device().await?;
    info!("Bootloader device detected: {}", bootloader_device);

    // Mount the bootloader device
    let mount_point = "/tmp/rpi-rp2-bootloader";
    fs::create_dir_all(mount_point).await?;

    info!("Mounting bootloader at {}...", mount_point);
    mount_bootloader(&bootloader_device, mount_point).await?;

    // Copy firmware to the mounted bootloader
    let firmware_dest = format!("{}/firmware.uf2", mount_point);
    info!("Copying firmware to bootloader...");
    if let Err(e) = fs::copy(&temp_file, &firmware_dest).await {
        error!("Failed to copy firmware to bootloader: {}", e);
        // Try to unmount before returning error
        let _ = unmount_bootloader(mount_point).await;
        return Err(e.into());
    }

    // Sync to ensure data is written
    sync_filesystem().await?;

    // Unmount the bootloader (device will reboot automatically)
    info!("Unmounting bootloader...");
    unmount_bootloader(mount_point).await?;

    // Wait for device to reboot and reconnect
    sleep(Duration::from_secs(5)).await;

    // Move to deployed directory
    fs::create_dir_all(DEPLOYED_DIR).await?;
    let deployed_file = format!("{}/moonblokz_{}.uf2", DEPLOYED_DIR, version_info.version);
    fs::rename(&temp_file, &deployed_file).await?;

    // Clean up old versions
    cleanup_old_node_versions(version_info.version).await?;

    info!("Node firmware updated successfully to version {}", version_info.version);

    Ok(())
}

async fn check_and_update_probe(config: &Config) -> Result<()> {
    // Fetch version info
    let version_url = format!("{}/version.json", config.probe_firmware_url);
    let response = reqwest::get(&version_url).await?;
    let version_info: VersionInfo = response.json().await?;

    // Determine current version
    let current_version = get_current_probe_version().await?;

    info!("Probe - Current: {}, Latest: {}", current_version, version_info.version);

    if version_info.version <= current_version {
        return Ok(());
    }

    info!("Updating probe to version {}...", version_info.version);

    // Download new binary
    let binary_url = format!("{}/moonblokz_probe_{}", config.probe_firmware_url, version_info.version);
    let response = reqwest::get(&binary_url).await?;
    let binary_data = response.bytes().await?;

    // Verify CRC32
    let computed_crc = crc32fast::hash(&binary_data);
    let expected_crc =
        u32::from_str_radix(&version_info.crc32, 16).map_err(|_| anyhow::anyhow!("Invalid CRC32 format in version.json: {}", version_info.crc32))?;

    if computed_crc != expected_crc {
        return Err(anyhow::anyhow!("CRC32 mismatch: expected {:x}, got {:x}", expected_crc, computed_crc));
    }

    // Save to deployed directory
    fs::create_dir_all(DEPLOYED_DIR).await?;
    let new_binary = format!("{}/moonblokz_probe_{}", DEPLOYED_DIR, version_info.version);
    fs::write(&new_binary, &binary_data).await?;

    debug!("Wrote new probe binary to {}", new_binary);

    // Set executable bit
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&new_binary).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&new_binary, perms).await?;
    }

    // Update start.sh
    let start_script = format!(
        "#!/bin/bash\n# Auto-generated start script\nexec {} --config config.toml\n",
        std::fs::canonicalize(&new_binary)?.display()
    );
    fs::write("start.sh", start_script).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata("start.sh").await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions("start.sh", perms).await?;
    }

    // Clean up old versions
    cleanup_old_probe_versions(version_info.version).await?;

    info!("Probe updated successfully to version {}", version_info.version);
    info!("Rebooting in 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    // Reboot
    reboot_system().await?;

    Ok(())
}

async fn get_current_node_version() -> Result<u32> {
    let mut entries = fs::read_dir(DEPLOYED_DIR).await?;

    while let Some(entry) = entries.next_entry().await? {
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if filename_str.starts_with("moonblokz_") && filename_str.ends_with(".uf2") {
            // Extract version number
            let version_str = filename_str.trim_start_matches("moonblokz_").trim_end_matches(".uf2");

            if let Ok(version) = version_str.parse::<u32>() {
                return Ok(version);
            }
        }
    }

    Ok(0) // No version found
}

async fn get_current_probe_version() -> Result<u32> {
    let mut entries = fs::read_dir(DEPLOYED_DIR).await?;

    while let Some(entry) = entries.next_entry().await? {
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if filename_str.starts_with("moonblokz_probe_") {
            // Extract version number
            let version_str = filename_str.trim_start_matches("moonblokz_probe_");

            if let Ok(version) = version_str.parse::<u32>() {
                return Ok(version);
            }
        }
    }

    Ok(0) // No version found
}

async fn cleanup_old_node_versions(current: u32) -> Result<()> {
    let mut entries = fs::read_dir(DEPLOYED_DIR).await?;

    while let Some(entry) = entries.next_entry().await? {
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if filename_str.starts_with("moonblokz_") && filename_str.ends_with(".uf2") {
            let version_str = filename_str.trim_start_matches("moonblokz_").trim_end_matches(".uf2");

            if let Ok(version) = version_str.parse::<u32>() {
                if version < current {
                    fs::remove_file(entry.path()).await?;
                    info!("Removed old node firmware version {}", version);
                }
            }
        }
    }

    Ok(())
}

async fn cleanup_old_probe_versions(current: u32) -> Result<()> {
    let mut entries = fs::read_dir(DEPLOYED_DIR).await?;

    while let Some(entry) = entries.next_entry().await? {
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if filename_str.starts_with("moonblokz_probe_") {
            let version_str = filename_str.trim_start_matches("moonblokz_probe_");

            if let Ok(version) = version_str.parse::<u32>() {
                if version < current {
                    fs::remove_file(entry.path()).await?;
                    info!("Removed old probe version {}", version);
                }
            }
        }
    }

    Ok(())
}

/// Wait for the RP2040 bootloader device to appear in /dev
async fn wait_for_bootloader_device() -> Result<String> {
    const MAX_WAIT_SECONDS: u64 = 30;
    const CHECK_INTERVAL_MS: u64 = 500;

    let max_attempts = (MAX_WAIT_SECONDS * 1000) / CHECK_INTERVAL_MS;

    for attempt in 0..max_attempts {
        // Check for block devices that might be the RP2040 bootloader
        // The RP2040 bootloader appears as a USB mass storage device
        if let Ok(mut entries) = fs::read_dir("/dev").await {
            while let Some(entry) = entries.next_entry().await.ok().flatten() {
                let filename = entry.file_name();
                let filename_str = filename.to_string_lossy();

                // Look for sdX or sdXN patterns (USB mass storage)
                if filename_str.starts_with("sd") && filename_str.len() >= 3 {
                    let device_path = format!("/dev/{}", filename_str);

                    // Check if this is the RP2040 bootloader by checking filesystem label
                    if is_rp2040_bootloader(&device_path).await {
                        return Ok(device_path);
                    }
                }
            }
        }

        if attempt < max_attempts - 1 {
            sleep(Duration::from_millis(CHECK_INTERVAL_MS)).await;
        }
    }

    Err(anyhow::anyhow!("Timeout waiting for bootloader device to appear"))
}

/// Check if a device is the RP2040 bootloader by examining its properties
async fn is_rp2040_bootloader(device_path: &str) -> bool {
    // Use blkid to check the filesystem label
    match Command::new("blkid")
        .arg("-s")
        .arg("LABEL")
        .arg("-o")
        .arg("value")
        .arg(device_path)
        .output()
        .await
    {
        Ok(output) => {
            if output.status.success() {
                let label = String::from_utf8_lossy(&output.stdout);
                let label = label.trim();
                // RP2040 bootloader typically has label "RPI-RP2"
                return label == "RPI-RP2" || label == "RPI-RP2";
            }
            false
        }
        Err(_) => false,
    }
}

/// Mount the bootloader device at the specified mount point
async fn mount_bootloader(device: &str, mount_point: &str) -> Result<()> {
    let status = Command::new("sudo")
        .arg("mount")
        .arg("-t")
        .arg("vfat")
        .arg(device)
        .arg(mount_point)
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to mount bootloader device"));
    }

    Ok(())
}

/// Unmount the bootloader device
async fn unmount_bootloader(mount_point: &str) -> Result<()> {
    let status = Command::new("sudo").arg("umount").arg(mount_point).status().await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to unmount bootloader device"));
    }

    Ok(())
}

/// Sync filesystem to ensure all data is written to disk
async fn sync_filesystem() -> Result<()> {
    let status = Command::new("sync").status().await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to sync filesystem"));
    }

    Ok(())
}

pub async fn reboot_system() -> Result<()> {
    let status = Command::new("sudo").arg("reboot").status().await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Reboot command failed"));
    }

    Ok(())
}
