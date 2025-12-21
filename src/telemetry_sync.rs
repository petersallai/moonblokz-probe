use crate::command_executor::{self, Command};
use crate::config::Config;
use crate::log_entry::LogEntry;
use crate::usb_manager::UsbHandle;
use anyhow::Result;
use log::{debug, error, info, warn};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 60000;

#[derive(Debug, Serialize)]
struct UploadRequest {
    logs: Vec<LogEntry>,
}

pub async fn run(
    config: Arc<Config>,
    buffer: Arc<RwLock<Vec<LogEntry>>>,
    upload_interval: Arc<RwLock<Duration>>,
    filter_string: Arc<RwLock<String>>,
    usb_handle: UsbHandle,
) -> Result<()> {
    let client = reqwest::Client::builder().use_rustls_tls().build()?;

    let mut backoff_ms = INITIAL_BACKOFF_MS;

    loop {
        let interval_duration = *upload_interval.read().await;

        sleep(interval_duration).await;

        match upload_telemetry(&client, &config, &buffer, &filter_string, &upload_interval, &usb_handle).await {
            Ok(_) => {
                backoff_ms = INITIAL_BACKOFF_MS;
            }
            Err(e) => {
                error!("Telemetry upload error: {}. Retrying in {}ms...", e, backoff_ms);
                sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
            }
        }
    }
}

async fn upload_telemetry(
    client: &reqwest::Client,
    config: &Config,
    buffer: &Arc<RwLock<Vec<LogEntry>>>,
    filter_string: &Arc<RwLock<String>>,
    upload_interval: &Arc<RwLock<Duration>>,
    usb_handle: &UsbHandle,
) -> Result<()> {
    // Prepare request with buffered logs
    let logs = {
        let buf = buffer.read().await;
        buf.clone()
    };

    // Always upload, even with empty logs - hub response may contain commands
    debug!("Uploading {} log entries to hub", logs.len());

    let request_body = UploadRequest { logs };

    // Send request
    let url = format!("{}/update", config.server_url);
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("X-Node-ID", config.node_id.to_string())
        .header("X-Api-Key", &config.api_key)
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        warn!("Upload failed with status: {}", status);
        return Err(anyhow::anyhow!("Non-success status: {}", status));
    }

    info!("Successfully uploaded telemetry");

    // Parse response commands
    let commands: Vec<Command> = match response.json().await {
        Ok(cmds) => cmds,
        Err(e) => {
            warn!("Failed to parse response commands: {}. Logs considered delivered.", e);
            // Clear buffer anyway since logs were delivered
            buffer.write().await.clear();
            return Ok(());
        }
    };

    // Clear buffer after successful upload
    buffer.write().await.clear();

    // Execute commands
    for command in commands {
        if let Err(e) = command_executor::execute_command(command, config, filter_string, upload_interval, usb_handle).await {
            error!("Command execution error: {}", e);
        }
    }

    Ok(())
}
