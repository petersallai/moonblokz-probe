use crate::config::Config;
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
    #[serde(default)]
    crc32: String,
    #[serde(default)]
    checksum: String,
}

pub async fn run_node_update(config: Arc<Config>) -> Result<()> {
    // Check on startup
    if let Err(e) = check_and_update_node_firmware(&config).await {
        error!("Node firmware update check failed: {}", e);
    }
    
    loop {
        sleep(Duration::from_secs(CHECK_INTERVAL_SECONDS)).await;
        
        if let Err(e) = check_and_update_node_firmware(&config).await {
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

async fn check_and_update_node_firmware(config: &Config) -> Result<()> {
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
    
    // Download new firmware
    let firmware_url = format!("{}/moonblokz_{}.uf2", config.node_firmware_url, version_info.version);
    let response = reqwest::get(&firmware_url).await?;
    let firmware_data = response.bytes().await?;
    
    // Verify checksum
    let computed_crc = crc32fast::hash(&firmware_data);
    let expected_crc = u32::from_str_radix(&version_info.crc32, 16)
        .unwrap_or_else(|_| {
            warn!("Could not parse CRC32, skipping verification");
            computed_crc
        });
    
    if computed_crc != expected_crc {
        return Err(anyhow::anyhow!("CRC32 mismatch: expected {:x}, got {:x}", expected_crc, computed_crc));
    }
    
    // Save to temporary file
    let temp_file = format!("/tmp/moonblokz_{}.uf2", version_info.version);
    fs::write(&temp_file, &firmware_data).await?;
    
    // Enter bootloader mode
    info!("Entering bootloader mode...");
    send_usb_command(&config.usb_port, "/BS").await?;
    
    // Wait for bootloader device to appear
    sleep(Duration::from_secs(5)).await;
    
    // Copy firmware (this requires the bootloader to be mounted)
    // In a real implementation, we would detect and mount the device
    // For now, assume it's mounted at a known location
    let bootloader_path = "/media/RPI-RP2";
    if let Err(e) = fs::copy(&temp_file, format!("{}/firmware.uf2", bootloader_path)).await {
        error!("Failed to copy firmware to bootloader: {}", e);
        return Err(e.into());
    }
    
    // Wait for device to reboot
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
    
    // Verify checksum if provided
    if !version_info.checksum.is_empty() {
        let computed_crc = crc32fast::hash(&binary_data);
        let expected_crc = u32::from_str_radix(&version_info.checksum, 16)
            .unwrap_or_else(|_| {
                warn!("Could not parse checksum, skipping verification");
                computed_crc
            });
        
        if computed_crc != expected_crc {
            return Err(anyhow::anyhow!("Checksum mismatch"));
        }
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
            let version_str = filename_str
                .trim_start_matches("moonblokz_")
                .trim_end_matches(".uf2");
            
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
            let version_str = filename_str
                .trim_start_matches("moonblokz_")
                .trim_end_matches(".uf2");
            
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

async fn send_usb_command(port: &str, command: &str) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    use tokio_serial::SerialPortBuilderExt;
    
    let mut port = tokio_serial::new(port, 115200)
        .open_native_async()?;
    
    port.write_all(format!("{}\r\n", command).as_bytes()).await?;
    port.flush().await?;
    
    Ok(())
}

pub async fn reboot_system() -> Result<()> {
    let status = Command::new("sudo")
        .arg("reboot")
        .status()
        .await?;
    
    if !status.success() {
        return Err(anyhow::anyhow!("Reboot command failed"));
    }
    
    Ok(())
}
