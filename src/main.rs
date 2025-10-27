mod config;
mod log_entry;
mod usb_manager;
mod usb_collector;
mod telemetry_sync;
mod update_manager;
mod command_executor;
mod error;

use anyhow::Result;
use clap::Parser;
use log::{error, info};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;

use config::Config;
use log_entry::LogEntry;
use usb_manager::{UsbManager, UsbHandle};

#[derive(Parser, Debug)]
#[command(name = "moonblokz-probe")]
#[command(about = "MoonBlokz Probe - Bridge between RP2040 node and telemetry infrastructure")]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Load configuration
    let config = Config::load(&args.config)?;
    
    // Initialize logger with level from config
    let log_level = match config.log_level.to_lowercase().as_str() {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };
    
    simple_logger::SimpleLogger::new()
        .with_level(log_level)
        .with_utc_timestamps()
        .init()
        .unwrap();
    
    info!("Loaded configuration from {:?}", args.config);
    info!("Node ID: {}", config.node_id);
    info!("USB Port: {}", config.usb_port);
    info!("Server URL: {}", config.server_url);
    info!("Upload interval: {}s", config.upload_interval_seconds);
    info!("Buffer size: {}", config.buffer_size);
    
    // Create channels for USB communication
    let (usb_cmd_tx, usb_cmd_rx) = mpsc::channel(32);
    let (usb_msg_tx, usb_msg_rx) = mpsc::channel(100);
    
    // Create USB handle for sending commands
    let usb_handle = UsbHandle::new(usb_cmd_tx);
    
    // Shared state
    let buffer = Arc::new(RwLock::new(Vec::<LogEntry>::new()));
    let filter_string = Arc::new(RwLock::new(config.filter_string.clone()));
    let upload_interval = Arc::new(RwLock::new(Duration::from_secs(config.upload_interval_seconds)));
    
    // Clone references for tasks
    let buffer_usb = Arc::clone(&buffer);
    let buffer_sync = Arc::clone(&buffer);
    let filter_usb = Arc::clone(&filter_string);
    let interval_sync = Arc::clone(&upload_interval);
    let config_sync = Arc::new(config.clone());
    let config_usb = Arc::clone(&config_sync);
    let config_node_update = Arc::clone(&config_sync);
    let config_probe_update = Arc::clone(&config_sync);
    let usb_handle_cmd = usb_handle.clone();
    
    // Spawn USB manager task
    let usb_manager = UsbManager::new(config.usb_port.clone(), usb_cmd_rx, usb_msg_tx);
    let usb_task = tokio::spawn(async move {
        usb_manager.run().await
    });
    
    // Spawn USB log collector task (receives messages from USB manager)
    let collector_task = tokio::spawn(async move {
        usb_collector::run(config_usb, buffer_usb, filter_usb, usb_msg_rx).await
    });
    
    // Spawn telemetry sync task
    let sync_task = tokio::spawn(async move {
        telemetry_sync::run(config_sync, buffer_sync, interval_sync, filter_string, usb_handle_cmd).await
    });
    
    // Spawn node firmware update manager
    let node_update_task = tokio::spawn(async move {
        update_manager::run_node_update(config_node_update).await
    });
    
    // Spawn probe self-update manager
    let probe_update_task = tokio::spawn(async move {
        update_manager::run_probe_update(config_probe_update).await
    });
    
    // Wait for any task to complete (they should run indefinitely)
    tokio::select! {
        result = usb_task => {
            error!("USB manager task ended: {:?}", result);
        }
        result = collector_task => {
            error!("USB collector task ended: {:?}", result);
        }
        result = sync_task => {
            error!("Telemetry sync task ended: {:?}", result);
        }
        result = node_update_task => {
            error!("Node update task ended: {:?}", result);
        }
        result = probe_update_task => {
            error!("Probe update task ended: {:?}", result);
        }
    }
    
    Ok(())
}
