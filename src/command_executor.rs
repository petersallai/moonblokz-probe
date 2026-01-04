use crate::config::Config;
use crate::update_manager;
use crate::usb_manager::UsbHandle;
use anyhow::Result;
use log::{error, info, warn};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CommandParameters {
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
    #[serde(default)]
    sequence: u32,
}

#[derive(Debug, Deserialize)]
pub struct Command {
    pub command: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

pub async fn execute_command(
    command: Command,
    _config: &Config,
    filter_string: &Arc<RwLock<String>>,
    _upload_interval: &Arc<RwLock<Duration>>,
    usb_handle: &UsbHandle,
) -> Result<()> {
    info!("Executing command: {}", command.command);

    let params: CommandParameters = serde_json::from_value(command.parameters).unwrap_or_else(|_| CommandParameters {
        level: String::new(),
        log_level: String::new(),
        value: String::new(),
        log_filter: String::new(),
        command: String::new(),
        sequence: 0,
    });

    match command.command.as_str() {
        "set_log_level" => {
            let level = if !params.log_level.is_empty() { &params.log_level } else { &params.level };

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

            usb_handle.send_command(usb_command.to_string()).await?;
            info!("Set log level to {}", level);
        }

        "set_log_filter" => {
            let new_filter = if !params.log_filter.is_empty() { params.log_filter } else { params.value };

            info!("Setting filter to: {}", new_filter);
            *filter_string.write().await = new_filter;
        }

        "run_command" => {
            if !params.command.is_empty() {
                usb_handle.send_command(params.command).await?;
            } else if !params.value.is_empty() {
                usb_handle.send_command(params.value).await?;
            }
        }

        "update_node" => {
            info!("Triggering node firmware update...");
            if let Err(e) = update_manager::check_and_update_node_firmware(_config, usb_handle).await {
                error!("Node firmware update failed: {}", e);
            }
        }

        "update_probe" => {
            info!("Triggering probe self-update...");
            if let Err(e) = update_manager::check_and_update_probe(_config).await {
                error!("Probe update failed: {}", e);
            }
        }

        "reboot_probe" => {
            info!("Rebooting probe...");
            tokio::time::sleep(Duration::from_secs(2)).await;
            update_manager::reboot_system().await?;
        }

        "start_measurement" => {
            if params.sequence == 0 {
                warn!("start_measurement requires a non-zero sequence number");
                return Ok(());
            }

            let usb_command = format!("/M_{}_", params.sequence);
            info!("Starting measurement with sequence {}", params.sequence);
            usb_handle.send_command(usb_command).await?;
        }

        _ => {
            warn!("Unknown command: {}", command.command);
        }
    }

    Ok(())
}
