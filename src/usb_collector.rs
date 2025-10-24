use crate::config::Config;
use crate::log_entry::LogEntry;
use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tokio_serial::SerialPortBuilderExt;

const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 60000;

pub async fn run(
    config: Arc<Config>,
    buffer: Arc<RwLock<Vec<LogEntry>>>,
    filter_string: Arc<RwLock<String>>,
) -> Result<()> {
    let mut backoff_ms = INITIAL_BACKOFF_MS;
    
    loop {
        match connect_and_read(&config, &buffer, &filter_string).await {
            Ok(_) => {
                eprintln!("USB connection closed normally");
                backoff_ms = INITIAL_BACKOFF_MS;
            }
            Err(e) => {
                eprintln!("USB connection error: {}. Retrying in {}ms...", e, backoff_ms);
                sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
            }
        }
    }
}

async fn connect_and_read(
    config: &Config,
    buffer: &Arc<RwLock<Vec<LogEntry>>>,
    filter_string: &Arc<RwLock<String>>,
) -> Result<()> {
    // Open serial port
    let port = tokio_serial::new(&config.usb_port, 115200)
        .open_native_async()?;
    
    println!("Connected to USB port: {}", config.usb_port);
    
    let reader = BufReader::new(port);
    let mut lines = reader.lines();
    
    while let Some(line) = lines.next_line().await? {
        // Generate timestamp in ISO 8601 UTC format
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        
        // Apply filter
        let filter = filter_string.read().await;
        if !filter.is_empty() && !line.contains(filter.as_str()) {
            continue;
        }
        drop(filter);
        
        // Create log entry
        let entry = LogEntry::new(timestamp, line);
        
        // Add to buffer, removing oldest if needed
        let mut buf = buffer.write().await;
        if buf.len() >= config.buffer_size {
            buf.remove(0);
        }
        buf.push(entry);
    }
    
    Ok(())
}
