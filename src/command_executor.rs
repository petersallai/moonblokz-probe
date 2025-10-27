use crate::config::Config;
use crate::update_manager;
use anyhow::Result;
use log::{info, warn};
use serde::Deserialize;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tokio_serial::SerialPortBuilderExt;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CommandParameters {
    #[serde(default)]
    start_time: String,
    #[serde(default)]
    end_time: String,
    #[serde(default)]
    active_period: u64,
    #[serde(default)]
    inactive_period: u64,
    #[serde(default)]
    level: String,
    #[serde(default)]
    log_level: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    log_filter: String,
    #[serde(default)]
    command: String,
}

#[derive(Debug, Deserialize)]
pub struct Command {
    pub command: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

pub async fn execute_command(
    command: Command,
    config: &Config,
    filter_string: &Arc<RwLock<String>>,
) -> Result<()> {
    info!("Executing command: {}", command.command);
    
    let params: CommandParameters = serde_json::from_value(command.parameters)
        .unwrap_or_else(|_| CommandParameters {
            start_time: String::new(),
            end_time: String::new(),
            active_period: 0,
            inactive_period: 0,
            level: String::new(),
            log_level: String::new(),
            value: String::new(),
            log_filter: String::new(),
            command: String::new(),
        });
    
    match command.command.as_str() {
        "set_update_interval" => {
            // TODO: Implement dynamic scheduling based on time windows
            // For now, just use active_period as the new interval
            if params.active_period > 0 {
                info!("Setting upload interval to {} seconds", params.active_period);
                // This would need to be passed back to the main loop
                // For now, just log it
            }
        }
        
        "set_log_level" => {
            let level = if !params.log_level.is_empty() {
                &params.log_level
            } else {
                &params.level
            };
            
            let usb_command = match level.to_uppercase().as_str() {
                "TRACE" => "/LT",
                "DEBUG" => "/LD",
                "INFO" => "/LI",
                "WARN" => "/LW",
                "ERROR" => "/LE",
                _ => {
                    warn!("Unknown log level: {}", level);
                    return Ok(());
                }
            };
            
            send_usb_command(&config.usb_port, usb_command).await?;
            info!("Set log level to {}", level);
        }
        
        "set_filter" => {
            let new_filter = if !params.log_filter.is_empty() {
                params.log_filter
            } else {
                params.value
            };
            
            info!("Setting filter to: {}", new_filter);
            *filter_string.write().await = new_filter;
        }
        
        "run_command" => {
            if !params.command.is_empty() {
                send_usb_command(&config.usb_port, &params.command).await?;
            } else if !params.value.is_empty() {
                send_usb_command(&config.usb_port, &params.value).await?;
            }
        }
        
        "update_node" => {
            info!("Triggering node firmware update...");
            // In a real implementation, we would signal the update manager
            // For now, the update manager runs on its own schedule
        }
        
        "update_probe" => {
            info!("Triggering probe self-update...");
            // In a real implementation, we would signal the update manager
            // For now, the update manager runs on its own schedule
        }
        
        "reboot_probe" => {
            info!("Rebooting probe...");
            tokio::time::sleep(Duration::from_secs(2)).await;
            update_manager::reboot_system().await?;
        }
        
        _ => {
            warn!("Unknown command: {}", command.command);
        }
    }
    
    Ok(())
}

async fn send_usb_command(port: &str, command: &str) -> Result<()> {
    let mut port = tokio_serial::new(port, 115200)
        .open_native_async()?;
    
    port.write_all(format!("{}\r\n", command).as_bytes()).await?;
    port.flush().await?;
    
    Ok(())
}
