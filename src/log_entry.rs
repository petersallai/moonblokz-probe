use serde::{Deserialize, Serialize};

/// A single log entry captured from the RP2040.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// ISO 8601 UTC timestamp
    pub timestamp: String,
    /// Original log line including [LEVEL]
    pub message: String,
}

impl LogEntry {
    pub fn new(timestamp: String, message: String) -> Self {
        Self { timestamp, message }
    }
}
