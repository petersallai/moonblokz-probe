use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
}

pub struct LogBuffer {
    entries: VecDeque<LogEntry>,
    max_size: usize,
}

impl LogBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_size,
        }
    }
    
    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_size {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }
    
    pub fn drain(&mut self) -> Vec<LogEntry> {
        self.entries.drain(..).collect()
    }
    
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerCommand {
    pub command: String,
    #[serde(flatten)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetUpdateIntervalPayload {
    pub start_time: String,
    pub end_time: String,
    pub active_period: u64,
    pub inactive_period: u64,
}

#[derive(Debug, Clone)]
pub struct UpdateInterval {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub active_period: u64,
    pub inactive_period: u64,
    pub default_interval: u64,
}

impl Default for UpdateInterval {
    fn default() -> Self {
        Self {
            start_time: None,
            end_time: None,
            active_period: 60,
            inactive_period: 60,
            default_interval: 60,
        }
    }
}

impl UpdateInterval {
    pub fn get_current_interval(&self) -> u64 {
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            let now = Utc::now();
            if now >= start && now <= end {
                // Within the time window, alternate between active and inactive
                let elapsed = (now - start).num_seconds() as u64;
                let cycle = self.active_period + self.inactive_period;
                let position = elapsed % cycle;
                
                if position < self.active_period {
                    return self.active_period;
                } else {
                    return self.inactive_period;
                }
            }
        }
        self.default_interval
    }
}

#[derive(Debug, Serialize)]
pub struct TelemetryPayload {
    pub logs: Vec<LogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    pub version: u32,
    pub crc32: String,
}
