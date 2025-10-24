use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(name = "moonblokz-probe", version, about)]
pub struct Cli {
    /// Path to configuration file
    #[clap(long, default_value = "./config.toml")]
    pub config: PathBuf,
    
    /// Override USB serial port path
    #[clap(long)]
    pub usb_port: Option<String>,
    
    /// Override telemetry server URL
    #[clap(long)]
    pub server_url: Option<String>,
    
    /// Override node ID
    #[clap(long)]
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub usb_port: String,
    pub server_url: String,
    pub api_key: String,
    pub node_id: String,
    pub node_firmware_url: String,
    pub probe_firmware_url: String,
}

pub fn load_config(cli: &Cli) -> Result<Config> {
    let config_content = fs::read_to_string(&cli.config)
        .with_context(|| format!("Failed to read config file: {:?}", cli.config))?;
    
    let mut config: Config = toml::from_str(&config_content)
        .context("Failed to parse config file")?;
    
    // Apply CLI overrides
    if let Some(ref usb_port) = cli.usb_port {
        config.usb_port = usb_port.clone();
    }
    
    if let Some(ref server_url) = cli.server_url {
        config.server_url = server_url.clone();
    }
    
    if let Some(ref node_id) = cli.node_id {
        config.node_id = node_id.clone();
    }
    
    Ok(config)
}
