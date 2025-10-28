use crate::config::Config;
use crate::log_entry::LogEntry;
use crate::usb_manager::UsbMessage;
use anyhow::Result;
use chrono::Utc;
use log::{info, trace};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub async fn run(
    config: Arc<Config>,
    buffer: Arc<RwLock<Vec<LogEntry>>>,
    filter_string: Arc<RwLock<String>>,
    mut usb_rx: mpsc::Receiver<UsbMessage>,
) -> Result<()> {
    info!("USB collector task started");
    
    while let Some(msg) = usb_rx.recv().await {
        match msg {
            UsbMessage::LineReceived(line) => {
                trace!("Processing line from USB: {}", line);
                
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
            UsbMessage::Connected => {
                info!("USB collector notified of connection");
            }
            UsbMessage::Disconnected => {
                info!("USB collector notified of disconnection");
            }
        }
    }
    
    Ok(())
}