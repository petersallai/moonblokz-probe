use anyhow::Result;
use log::{debug, error, info, trace, warn};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant};
use tokio_serial::SerialPortBuilderExt;

const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 60000;
const BOOTLOADER_RECOVERY_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// Commands that can be sent to the USB manager
#[derive(Debug, Clone)]
pub enum UsbCommand {
    /// Send a raw command to the USB port
    SendCommand(String),
}

/// Messages from USB manager to consumers
#[derive(Debug, Clone)]
pub enum UsbMessage {
    /// A line was received from the USB port
    LineReceived(String),
    /// Connection status changed
    Connected,
    Disconnected,
}

/// Manages the USB serial port connection and handles both reading and writing
pub struct UsbManager {
    port_path: String,
    command_rx: mpsc::Receiver<UsbCommand>,
    message_tx: mpsc::Sender<UsbMessage>,
}

impl UsbManager {
    pub fn new(
        port_path: String,
        command_rx: mpsc::Receiver<UsbCommand>,
        message_tx: mpsc::Sender<UsbMessage>,
    ) -> Self {
        Self {
            port_path,
            command_rx,
            message_tx,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut backoff_ms = INITIAL_BACKOFF_MS;
        let mut disconnected_since: Option<Instant> = None;
        let mut recovery_attempted = false;

        loop {
            match self.connect_and_handle().await {
                Ok(_) => {
                    // Connection was established then closed normally - start tracking disconnection
                    info!("USB connection closed normally");
                    backoff_ms = INITIAL_BACKOFF_MS;
                    disconnected_since = Some(Instant::now());
                    recovery_attempted = false;
                }
                Err(e) => {
                    error!(
                        "USB connection error: {}. Retrying in {}ms...",
                        e, backoff_ms
                    );
                    let _ = self.message_tx.send(UsbMessage::Disconnected).await;

                    // Track disconnection time
                    if disconnected_since.is_none() {
                        disconnected_since = Some(Instant::now());
                        recovery_attempted = false;
                    }

                    // Check if we've been disconnected for 5 minutes
                    if let Some(since) = disconnected_since {
                        let disconnected_duration = since.elapsed();
                        if disconnected_duration.as_secs() >= BOOTLOADER_RECOVERY_TIMEOUT_SECS
                            && !recovery_attempted
                        {
                            warn!(
                                "USB disconnected for {} seconds, attempting bootloader recovery...",
                                disconnected_duration.as_secs()
                            );
                            recovery_attempted = true;

                            // Try to recover node stuck in bootloader mode
                            if let Err(e) =
                                crate::update_manager::recover_node_from_bootloader().await
                            {
                                error!("Bootloader recovery failed: {}", e);
                            } else {
                                info!(
                                    "Bootloader recovery completed, resetting connection tracking"
                                );
                                disconnected_since = None;
                                backoff_ms = INITIAL_BACKOFF_MS;
                            }
                        }
                    }

                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                }
            }

            // Reset disconnection tracking if we successfully connect next iteration
            // This happens implicitly - connect_and_handle() will succeed and Connected message is sent
        }
    }

    async fn connect_and_handle(&mut self) -> Result<()> {
        // Open serial port
        let port = tokio_serial::new(&self.port_path, 115200).open_native_async()?;

        info!("Connected to USB port: {}", self.port_path);
        let _ = self.message_tx.send(UsbMessage::Connected).await;

        // Note: disconnected_since is reset in the caller when connection succeeds

        // Split port into read and write halves
        let (reader, mut writer) = tokio::io::split(port);
        let mut reader = BufReader::new(reader);
        let mut line_buffer = String::new();

        loop {
            tokio::select! {
                // Handle incoming lines from USB
                result = reader.read_line(&mut line_buffer) => {
                    match result {
                        Ok(0) => {
                            // EOF - connection closed
                            info!("USB connection closed");
                            break;
                        }
                        Ok(_) => {
                            // Remove trailing newline
                            let line = line_buffer.trim_end().to_string();
                            if !line.is_empty() {
                                trace!("Received line from USB: {}", line);
                                let _ = self.message_tx.send(UsbMessage::LineReceived(line)).await;
                            }
                            line_buffer.clear();
                        }
                        Err(e) => {
                            error!("Error reading from USB: {}", e);
                            return Err(e.into());
                        }
                    }
                }

                // Handle commands to send to USB
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        UsbCommand::SendCommand(command) => {
                            debug!("Sending command to USB: {}", command);
                            if let Err(e) = writer.write_all(format!("{}\r\n", command).as_bytes()).await {
                                error!("Error writing to USB: {}", e);
                                return Err(e.into());
                            }
                            if let Err(e) = writer.flush().await {
                                error!("Error flushing USB: {}", e);
                                return Err(e.into());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Handle for sending commands to the USB manager
#[derive(Clone)]
pub struct UsbHandle {
    command_tx: mpsc::Sender<UsbCommand>,
}

impl UsbHandle {
    pub fn new(command_tx: mpsc::Sender<UsbCommand>) -> Self {
        Self { command_tx }
    }

    /// Send a command to the USB port
    pub async fn send_command(&self, command: String) -> Result<()> {
        self.command_tx
            .send(UsbCommand::SendCommand(command))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send USB command: {}", e))
    }
}
