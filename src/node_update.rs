use crate::config::Config;
use crate::types::VersionInfo;
use anyhow::{Context, Result};
use log::{info, warn};
use std::fs;
use std::path::PathBuf;

pub async fn check_and_update(config: &Config) -> Result<()> {
    info!("Checking for node firmware updates");
    
    // Fetch latest version info
    let version_url = format!("{}/version.json", config.node_firmware_url);
    let client = reqwest::Client::new();
    let version_info: VersionInfo = client
        .get(&version_url)
        .send()
        .await
        .context("Failed to fetch version info")?
        .json()
        .await
        .context("Failed to parse version info")?;
    
    info!("Latest node firmware version: {}", version_info.version);
    
    // Get current version from deployed directory
    let current_version = get_current_node_version()?;
    info!("Current node firmware version: {}", current_version);
    
    if version_info.version <= current_version {
        info!("Node firmware is up to date");
        return Ok(());
    }
    
    info!(
        "Updating node firmware from version {} to {}",
        current_version, version_info.version
    );
    
    // Download new firmware
    let firmware_url = format!(
        "{}/moonblokz_{}.uf2",
        config.node_firmware_url, version_info.version
    );
    
    let firmware_data = client
        .get(&firmware_url)
        .send()
        .await
        .context("Failed to download firmware")?
        .bytes()
        .await
        .context("Failed to read firmware data")?;
    
    // Verify CRC32
    let crc = crc32fast::hash(&firmware_data);
    let crc_hex = format!("{:08x}", crc);
    
    if crc_hex != version_info.crc32 {
        anyhow::bail!(
            "CRC32 mismatch: expected {}, got {}",
            version_info.crc32,
            crc_hex
        );
    }
    
    info!("Firmware CRC32 verified");
    
    // Save firmware to temp file
    let temp_path = PathBuf::from("/tmp/firmware.uf2");
    fs::write(&temp_path, firmware_data).context("Failed to write firmware to temp file")?;
    
    // TODO: Switch node to bootloader mode (send /BS\r\n via USB)
    // This would require access to the serial port, which might be in use
    warn!("Manual step required: Put node in bootloader mode and mount UF2 drive");
    warn!("Then copy {} to the mounted drive", temp_path.display());
    
    // In a full implementation, we would:
    // 1. Send /BS\r\n to USB port
    // 2. Poll for RP2040 bootloader device
    // 3. Mount the device (sudo mount /dev/sdX /mnt/rp2)
    // 4. Copy the UF2 file
    // 5. The device will auto-unmount and reboot
    // 6. Clean up old firmware from deployed/
    // 7. Move new firmware to deployed/ with proper naming
    
    info!("Node firmware update prepared (manual completion required)");
    
    Ok(())
}

fn get_current_node_version() -> Result<u32> {
    let deployed_dir = PathBuf::from("deployed");
    
    if !deployed_dir.exists() {
        fs::create_dir_all(&deployed_dir)?;
        return Ok(0);
    }
    
    // Look for moonblokz_*.uf2 files
    let entries = fs::read_dir(&deployed_dir)?;
    let mut max_version = 0;
    
    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();
        
        if filename_str.starts_with("moonblokz_") && filename_str.ends_with(".uf2") {
            // Extract version number
            let version_str = filename_str
                .trim_start_matches("moonblokz_")
                .trim_end_matches(".uf2");
            
            if let Ok(version) = version_str.parse::<u32>() {
                max_version = max_version.max(version);
            }
        }
    }
    
    Ok(max_version)
}
