use crate::config::Config;
use crate::update_manager;
use crate::usb_manager::UsbHandle;
use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{error, info, warn};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;

/// Schedule for upload intervals with active/inactive periods
#[derive(Debug, Clone)]
pub struct UploadSchedule {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub active_period: u64,
    pub inactive_period: u64,
}

impl UploadSchedule {
    /// Calculate the current upload interval based on whether we're in the active window
    pub fn current_interval(&self) -> u64 {
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            let now = Utc::now();
            if now >= start && now <= end {
                // We're in the active window
                return self.active_period;
            }
        }
        // Outside the active window (or no window defined)
        self.inactive_period
    }
}

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
    upload_interval: &Arc<RwLock<Duration>>,
    usb_handle: &UsbHandle,
) -> Result<()> {
    info!("Executing command: {}", command.command);

    let params: CommandParameters = serde_json::from_value(command.parameters).unwrap_or_else(|_| CommandParameters {
        start_time: String::new(),
        end_time: String::new(),
        active_period: 0,
        inactive_period: 0,
        level: String::new(),
        log_level: String::new(),
        value: String::new(),
        log_filter: String::new(),
        command: String::new(),
        sequence: 0,
    });

    match command.command.as_str() {
        "set_update_interval" => {
            // Parse time parameters
            let start_time = if !params.start_time.is_empty() {
                match DateTime::parse_from_rfc3339(&params.start_time) {
                    Ok(dt) => Some(dt.with_timezone(&Utc)),
                    Err(e) => {
                        error!("Failed to parse start_time '{}': {}", params.start_time, e);
                        None
                    }
                }
            } else {
                None
            };

            let end_time = if !params.end_time.is_empty() {
                match DateTime::parse_from_rfc3339(&params.end_time) {
                    Ok(dt) => Some(dt.with_timezone(&Utc)),
                    Err(e) => {
                        error!("Failed to parse end_time '{}': {}", params.end_time, e);
                        None
                    }
                }
            } else {
                None
            };

            // Validate periods
            if params.active_period == 0 && params.inactive_period == 0 {
                warn!("set_update_interval requires at least one period to be set");
                return Ok(());
            }

            // Create schedule
            let schedule = UploadSchedule {
                start_time,
                end_time,
                active_period: if params.active_period > 0 {
                    params.active_period
                } else {
                    params.inactive_period
                },
                inactive_period: if params.inactive_period > 0 {
                    params.inactive_period
                } else {
                    params.active_period
                },
            };

            // Calculate current interval based on schedule
            let current_interval_secs = schedule.current_interval();
            *upload_interval.write().await = Duration::from_secs(current_interval_secs);

            if let (Some(start), Some(end)) = (start_time, end_time) {
                info!(
                    "Set upload interval: active={}s (from {} to {}), inactive={}s. Current: {}s",
                    params.active_period,
                    start.to_rfc3339(),
                    end.to_rfc3339(),
                    params.inactive_period,
                    current_interval_secs
                );
            } else {
                info!("Set upload interval to {} seconds (no time window specified)", current_interval_secs);
            }
        }

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
