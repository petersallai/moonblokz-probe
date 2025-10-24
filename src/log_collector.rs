use crate::config::Config;
use crate::types::{LogBuffer, LogEntry};
use anyhow::{Context, Result};
use chrono::Utc;
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const BACKOFF_MULTIPLIER: u32 = 2;

pub async fn run(
    config: Config,
    log_buffer: Arc<Mutex<LogBuffer>>,
    log_filter: Arc<Mutex<String>>,
    serial_tx: Arc<Mutex<Option<SerialStream>>>,
) {
    let mut backoff = INITIAL_BACKOFF;
    
    loop {
        info!("Attempting to connect to USB port: {}", config.usb_port);
        
        match connect_and_read(&config, &log_buffer, &log_filter, &serial_tx).await {
            Ok(_) => {
                info!("USB connection closed normally");
                backoff = INITIAL_BACKOFF;
            }
            Err(e) => {
                error!("USB connection error: {}", e);
                warn!("Retrying in {:?}", backoff);
                sleep(backoff).await;
                
                // Exponential backoff
                backoff = std::cmp::min(backoff * BACKOFF_MULTIPLIER, MAX_BACKOFF);
            }
        }
    }
}

async fn connect_and_read(
    config: &Config,
    log_buffer: &Arc<Mutex<LogBuffer>>,
    log_filter: &Arc<Mutex<String>>,
    serial_tx: &Arc<Mutex<Option<SerialStream>>>,
) -> Result<()> {
    // Open serial port
    let port = tokio_serial::new(&config.usb_port, 115200)
        .open_native_async()
        .context("Failed to open serial port")?;
    
    info!("Connected to USB port: {}", config.usb_port);
    
    // Clone the port for sending commands
    let port_clone = tokio_serial::new(&config.usb_port, 115200)
        .open_native_async()
        .ok();
    
    {
        let mut tx = serial_tx.lock().await;
        *tx = port_clone;
    }
    
    let reader = BufReader::new(port);
    let mut lines = reader.lines();
    
    while let Ok(Some(line)) = lines.next_line().await {
        // Process the log line
        if is_valid_log_line(&line) {
            let timestamp = Utc::now().to_rfc3339();
            let entry = LogEntry {
                timestamp,
                message: line.to_string(),
            };
            
            // Check if line matches filter
            let filter = log_filter.lock().await.clone();
            let should_buffer = filter.is_empty() || line.contains(&filter);
            
            if should_buffer {
                let mut buffer = log_buffer.lock().await;
                buffer.push(entry);
            }
        }
    }
    
    Ok(())
}

fn is_valid_log_line(line: &str) -> bool {
    line.starts_with("[TRACE]")
        || line.starts_with("[DEBUG]")
        || line.starts_with("[INFO]")
        || line.starts_with("[WARN]")
        || line.starts_with("[ERROR]")
}
