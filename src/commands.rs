use crate::config::Config;
use crate::node_update;
use crate::probe_update;
use crate::types::{ServerCommand, SetUpdateIntervalPayload, UpdateInterval};
use anyhow::{Context, Result};
use chrono::DateTime;
use log::{info, warn};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio_serial::SerialStream;

pub async fn execute_command(
    cmd: ServerCommand,
    config: &Config,
    log_filter: &Arc<Mutex<String>>,
    update_interval: &Arc<Mutex<UpdateInterval>>,
    serial_tx: &Arc<Mutex<Option<SerialStream>>>,
) -> Result<()> {
    info!("Executing command: {}", cmd.command);
    
    match cmd.command.as_str() {
        "set_update_interval" => {
            let payload: SetUpdateIntervalPayload = serde_json::from_value(cmd.payload)
                .context("Invalid set_update_interval payload")?;
            
            let start_time = DateTime::parse_from_rfc3339(&payload.start_time)
                .context("Invalid start_time format")?
                .with_timezone(&chrono::Utc);
            
            let end_time = DateTime::parse_from_rfc3339(&payload.end_time)
                .context("Invalid end_time format")?
                .with_timezone(&chrono::Utc);
            
            let mut interval = update_interval.lock().await;
            interval.start_time = Some(start_time);
            interval.end_time = Some(end_time);
            interval.active_period = payload.active_period;
            interval.inactive_period = payload.inactive_period;
            
            info!(
                "Update interval configured: active={}, inactive={}",
                payload.active_period, payload.inactive_period
            );
        }
        
        "set_log_level" => {
            let level = cmd.payload.get("level")
                .and_then(|v| v.as_str())
                .context("Missing or invalid 'level' field")?;
            
            let usb_cmd = match level {
                "TRACE" => "/LT\r\n",
                "DEBUG" => "/LD\r\n",
                "INFO" => "/LI\r\n",
                "WARN" => "/LW\r\n",
                "ERROR" => "/LE\r\n",
                _ => anyhow::bail!("Invalid log level: {}", level),
            };
            
            send_usb_command(serial_tx, usb_cmd).await?;
            info!("Log level set to: {}", level);
        }
        
        "set_filter" => {
            let value = cmd.payload.get("value")
                .and_then(|v| v.as_str())
                .context("Missing or invalid 'value' field")?;
            
            let mut filter = log_filter.lock().await;
            *filter = value.to_string();
            
            info!("Log filter set to: {}", value);
        }
        
        "run_command" => {
            let value = cmd.payload.get("value")
                .and_then(|v| v.as_str())
                .context("Missing or invalid 'value' field")?;
            
            let usb_cmd = format!("{}\r\n", value);
            send_usb_command(serial_tx, &usb_cmd).await?;
            info!("Executed command on node: {}", value);
        }
        
        "update_node" => {
            info!("Starting node firmware update");
            tokio::spawn({
                let config = config.clone();
                async move {
                    if let Err(e) = node_update::check_and_update(&config).await {
                        log::error!("Node firmware update failed: {}", e);
                    }
                }
            });
        }
        
        "update_probe" => {
            info!("Starting probe self-update");
            tokio::spawn({
                let config = config.clone();
                async move {
                    if let Err(e) = probe_update::check_and_update(&config).await {
                        log::error!("Probe self-update failed: {}", e);
                    }
                }
            });
        }
        
        "reboot_probe" => {
            info!("Rebooting probe");
            std::process::Command::new("sudo")
                .arg("reboot")
                .spawn()
                .context("Failed to execute reboot command")?;
        }
        
        _ => {
            warn!("Unknown command: {}", cmd.command);
        }
    }
    
    Ok(())
}

async fn send_usb_command(
    serial_tx: &Arc<Mutex<Option<SerialStream>>>,
    command: &str,
) -> Result<()> {
    let mut tx = serial_tx.lock().await;
    
    if let Some(ref mut port) = *tx {
        port.write_all(command.as_bytes())
            .await
            .context("Failed to write to serial port")?;
        Ok(())
    } else {
        anyhow::bail!("Serial port not available")
    }
}
