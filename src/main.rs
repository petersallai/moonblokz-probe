mod config;
mod log_collector;
mod telemetry_sync;
mod node_update;
mod probe_update;
mod commands;
mod types;

use anyhow::Result;
use clap::Parser;
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    info!("Starting moonblokz-probe");
    
    // Parse command-line arguments
    let cli = config::Cli::parse();
    
    // Load configuration
    let config = config::load_config(&cli)?;
    info!("Configuration loaded successfully");
    
    // Shared state for log buffer
    let log_buffer = Arc::new(Mutex::new(types::LogBuffer::new(10000)));
    
    // Shared state for log filter
    let log_filter = Arc::new(Mutex::new(String::new()));
    
    // Shared state for update interval
    let update_interval = Arc::new(Mutex::new(types::UpdateInterval::default()));
    
    // Shared state for serial port path (for sending commands)
    let serial_tx = Arc::new(Mutex::new(None::<tokio_serial::SerialStream>));
    
    // Check for probe self-update on startup
    info!("Checking for probe self-update");
    if let Err(e) = probe_update::check_and_update(&config).await {
        log::error!("Probe self-update check failed: {}", e);
    }
    
    // Check for node firmware update on startup
    info!("Checking for node firmware update");
    if let Err(e) = node_update::check_and_update(&config).await {
        log::error!("Node firmware update check failed: {}", e);
    }
    
    // Spawn log collector task
    let collector_handle = {
        let config = config.clone();
        let log_buffer = Arc::clone(&log_buffer);
        let log_filter = Arc::clone(&log_filter);
        let serial_tx = Arc::clone(&serial_tx);
        
        tokio::spawn(async move {
            log_collector::run(config, log_buffer, log_filter, serial_tx).await
        })
    };
    
    // Spawn telemetry sync task
    let telemetry_handle = {
        let config = config.clone();
        let log_buffer = Arc::clone(&log_buffer);
        let log_filter = Arc::clone(&log_filter);
        let update_interval = Arc::clone(&update_interval);
        let serial_tx = Arc::clone(&serial_tx);
        
        tokio::spawn(async move {
            telemetry_sync::run(config, log_buffer, log_filter, update_interval, serial_tx).await
        })
    };
    
    info!("All tasks started successfully");
    
    // Wait for tasks to complete (they should run indefinitely)
    tokio::select! {
        _ = collector_handle => {
            log::error!("Log collector task terminated unexpectedly");
        }
        _ = telemetry_handle => {
            log::error!("Telemetry sync task terminated unexpectedly");
        }
    }
    
    Ok(())
}
