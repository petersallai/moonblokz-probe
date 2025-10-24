use crate::commands;
use crate::config::Config;
use crate::types::{LogBuffer, ServerCommand, TelemetryPayload, UpdateInterval};
use anyhow::{Context, Result};
use log::{error, info};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_serial::SerialStream;

pub async fn run(
    config: Config,
    log_buffer: Arc<Mutex<LogBuffer>>,
    log_filter: Arc<Mutex<String>>,
    update_interval: Arc<Mutex<UpdateInterval>>,
    serial_tx: Arc<Mutex<Option<SerialStream>>>,
) {
    let client = reqwest::Client::new();
    
    loop {
        // Get current interval
        let interval_secs = {
            let interval = update_interval.lock().await;
            interval.get_current_interval()
        };
        
        info!("Next telemetry sync in {} seconds", interval_secs);
        sleep(Duration::from_secs(interval_secs)).await;
        
        // Perform sync
        if let Err(e) = sync_telemetry(
            &config,
            &client,
            &log_buffer,
            &log_filter,
            &update_interval,
            &serial_tx,
        )
        .await
        {
            error!("Telemetry sync failed: {}", e);
            // Logs remain in buffer for retry
        }
    }
}

async fn sync_telemetry(
    config: &Config,
    client: &reqwest::Client,
    log_buffer: &Arc<Mutex<LogBuffer>>,
    log_filter: &Arc<Mutex<String>>,
    update_interval: &Arc<Mutex<UpdateInterval>>,
    serial_tx: &Arc<Mutex<Option<SerialStream>>>,
) -> Result<()> {
    // Collect logs from buffer
    let logs = {
        let mut buffer = log_buffer.lock().await;
        if buffer.len() == 0 {
            info!("No logs to sync");
            return Ok(());
        }
        buffer.drain()
    };
    
    info!("Syncing {} log entries", logs.len());
    
    let payload = TelemetryPayload { logs: logs.clone() };
    
    // Send POST request
    let response = client
        .post(&config.server_url)
        .header("X-Node-ID", &config.node_id)
        .header("X-Api-Key", &config.api_key)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("Failed to send telemetry request")?;
    
    if response.status() == 200 {
        info!("Telemetry sync successful");
        
        // Try to parse and execute commands from response
        if let Ok(commands) = response.json::<Vec<ServerCommand>>().await {
            info!("Received {} commands from server", commands.len());
            for cmd in commands {
                if let Err(e) = commands::execute_command(
                    cmd,
                    config,
                    log_filter,
                    update_interval,
                    serial_tx,
                )
                .await
                {
                    error!("Failed to execute command: {}", e);
                }
            }
        }
    } else {
        // Put logs back in buffer on failure
        let mut buffer = log_buffer.lock().await;
        for entry in logs.into_iter().rev() {
            buffer.push(entry);
        }
        
        anyhow::bail!(
            "Server returned non-200 status: {}",
            response.status()
        );
    }
    
    Ok(())
}
