use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub usb_port: String,
    pub server_url: String,
    pub api_key: String,
    pub node_id: u32,
    pub node_firmware_url: String,
    pub probe_firmware_url: String,
    #[serde(default = "default_upload_interval")]
    pub upload_interval_seconds: u64,
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    #[serde(default = "default_filter_string")]
    pub filter_string: String,
}

fn default_upload_interval() -> u64 {
    300
}

fn default_buffer_size() -> usize {
    10_000
}

fn default_filter_string() -> String {
    String::new()
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;
        
        Ok(config)
    }
}
