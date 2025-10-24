use crate::config::Config;
use crate::types::VersionInfo;
use anyhow::{Context, Result};
use log::info;
use std::fs;
use std::path::PathBuf;

pub async fn check_and_update(config: &Config) -> Result<()> {
    info!("Checking for probe self-updates");
    
    // Fetch latest version info
    let version_url = format!("{}/version.json", config.probe_firmware_url);
    let client = reqwest::Client::new();
    let version_info: VersionInfo = client
        .get(&version_url)
        .send()
        .await
        .context("Failed to fetch version info")?
        .json()
        .await
        .context("Failed to parse version info")?;
    
    info!("Latest probe version: {}", version_info.version);
    
    // Get current version from deployed directory
    let current_version = get_current_probe_version()?;
    info!("Current probe version: {}", current_version);
    
    if version_info.version <= current_version {
        info!("Probe is up to date");
        return Ok(());
    }
    
    info!(
        "Updating probe from version {} to {}",
        current_version, version_info.version
    );
    
    // Download new binary
    let binary_url = format!(
        "{}/moonblokz_probe_{}",
        config.probe_firmware_url, version_info.version
    );
    
    let binary_data = client
        .get(&binary_url)
        .send()
        .await
        .context("Failed to download probe binary")?
        .bytes()
        .await
        .context("Failed to read binary data")?;
    
    // Verify CRC32
    let crc = crc32fast::hash(&binary_data);
    let crc_hex = format!("{:08x}", crc);
    
    if crc_hex != version_info.crc32 {
        anyhow::bail!(
            "CRC32 mismatch: expected {}, got {}",
            version_info.crc32,
            crc_hex
        );
    }
    
    info!("Probe binary CRC32 verified");
    
    // Ensure deployed directory exists
    let deployed_dir = PathBuf::from("deployed");
    fs::create_dir_all(&deployed_dir)?;
    
    // Save new binary
    let new_binary_path = deployed_dir.join(format!("moonblokz_probe_{}", version_info.version));
    fs::write(&new_binary_path, binary_data)
        .context("Failed to write new probe binary")?;
    
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&new_binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&new_binary_path, perms)?;
    }
    
    info!("New probe binary saved to: {:?}", new_binary_path);
    
    // Remove old binaries
    let entries = fs::read_dir(&deployed_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();
        
        if filename_str.starts_with("moonblokz_probe_")
            && filename_str != format!("moonblokz_probe_{}", version_info.version)
        {
            info!("Removing old probe binary: {:?}", path);
            fs::remove_file(path)?;
        }
    }
    
    // Update start script
    let start_script_path = PathBuf::from("start.sh");
    let start_script_content = format!(
        "#!/bin/bash\n# This script is auto-generated. DO NOT EDIT.\n{} --config config.toml\n",
        new_binary_path.canonicalize()?.display()
    );
    
    fs::write(&start_script_path, start_script_content)
        .context("Failed to write start script")?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&start_script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&start_script_path, perms)?;
    }
    
    info!("Start script updated: {:?}", start_script_path);
    
    // Reboot to apply update
    info!("Rebooting to apply probe update");
    std::process::Command::new("sudo")
        .arg("reboot")
        .spawn()
        .context("Failed to execute reboot command")?;
    
    Ok(())
}

fn get_current_probe_version() -> Result<u32> {
    let deployed_dir = PathBuf::from("deployed");
    
    if !deployed_dir.exists() {
        fs::create_dir_all(&deployed_dir)?;
        return Ok(0);
    }
    
    // Look for moonblokz_probe_* files
    let entries = fs::read_dir(&deployed_dir)?;
    let mut max_version = 0;
    
    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();
        
        if filename_str.starts_with("moonblokz_probe_") {
            // Extract version number
            let version_str = filename_str.trim_start_matches("moonblokz_probe_");
            
            if let Ok(version) = version_str.parse::<u32>() {
                max_version = max_version.max(version);
            }
        }
    }
    
    Ok(max_version)
}
