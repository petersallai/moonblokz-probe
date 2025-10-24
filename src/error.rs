use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ProbeError {
    #[error("USB serial port error: {0}")]
    UsbError(#[from] tokio_serial::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Firmware update error: {0}")]
    FirmwareError(String),
    
    #[error("Command execution error: {0}")]
    CommandError(String),
}
