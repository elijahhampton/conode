use chrono::Utc;
use once_cell::sync::Lazy;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AsyncLogger {
    // Acts as primary channel for sending log entries from the logger
    // interface to the background worker task.
    sender: Sender<LogEntry>,
    // Maintains an in-memory cache of recent log entires (limited to 1000 entries)
    // Primarily used for UI display. Logs can be quickly retrieved without reading from disk.
    logs: Arc<Mutex<Vec<LogEntry>>>,
    // Enables real time log notifications to multiple subscribers.
    broadcast_tx: broadcast::Sender<LogEntry>,
}

pub static LOGGER: Lazy<Arc<Mutex<Option<AsyncLogger>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

impl AsyncLogger {
    /// Creates a new instance of the AsyncLogger.
    pub async fn new(log_file_path: &str, flush_interval: Duration) -> Self {
        let (sender, mut receiver): (Sender<LogEntry>, _) = channel(1000);
        let sender_clone: Sender<LogEntry> = sender.clone();
        let log_file_path = log_file_path.to_string();
        let logs = Arc::new(Mutex::new(Vec::new()));
        let logs_clone = Arc::clone(&logs);

        // Create broadcast channel
        let (broadcast_tx, _) = broadcast::channel(1000);
        let broadcast_tx_clone = broadcast_tx.clone();

        let file = Arc::new(Mutex::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_path)
                .expect("Failed to open log file"),
        ));

        let file_clone = Arc::clone(&file);

        // Spawn a thread checking for newly received logs and occasionally
        // flushing the log buffer on an interval.
        tokio::spawn(async move {
            let mut buffer = Vec::new();
            let mut interval = interval(flush_interval);

            loop {
                tokio::select! {
                    Some(entry) = receiver.recv() => {
                        // Store in memory for UI
                        let mut logs = logs_clone.lock().await;
                        logs.push(entry.clone());
                        if logs.len() > 1000 {
                            logs.remove(0);
                        }
                        drop(logs); // Explicitly drop the lock

                        // Broadcast the entry
                        let _ = broadcast_tx.send(entry.clone());

                        buffer.push(entry);
                        if buffer.len() >= 100 {
                            flush_logs(&file_clone, &mut buffer).await;
                        }
                    }
                    _ = interval.tick() => {
                        if !buffer.is_empty() {
                            flush_logs(&file_clone, &mut buffer).await;
                        }
                    }
                }
            }
        });

        AsyncLogger {
            sender: sender_clone,
            logs,
            broadcast_tx: broadcast_tx_clone,
        }
    }

    /// General logging function allow a LogLevel and message string.
    pub async fn log(&self, level: LogLevel, message: String) {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            message,
        };
        let _ = self.sender.send(entry).await;
    }

    /// Log using LogLevel::Debug.
    async fn debug(&self, message: String) {
        self.log(LogLevel::Debug, message).await;
    }

    /// Log using LogLevel::Info.
    async fn info(&self, message: String) {
        self.log(LogLevel::Info, message).await;
    }

    /// Log using LogLevel::Warning.
    async fn warning(&self, message: String) {
        self.log(LogLevel::Warning, message).await;
    }

    /// Log using LogLevel::Error.
    async fn error(&self, message: String) {
        self.log(LogLevel::Error, message).await;
    }

    pub fn get_logs_arc(&self) -> Arc<Mutex<Vec<LogEntry>>> {
        Arc::clone(&self.logs)
    }

    /// Retrieve all of the logs currently held in the buffer.
    pub async fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.lock().await.clone()
    }

    // Subscribe to real-time log updates.
    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.broadcast_tx.subscribe()
    }
}

/// Flush all of the logs currently in the buffer and clean the file.
async fn flush_logs(file: &Arc<Mutex<File>>, buffer: &mut Vec<LogEntry>) {
    let mut file_guard = file.lock().await;
    for entry in buffer.drain(..) {
        let _ = writeln!(
            file_guard,
            "[{}] {:?}: {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            entry.level,
            entry.message
        );
    }
    let _ = file_guard.flush();
}

/// Initialize the logger.
pub async fn initialize_logger(log_file_path: &str, flush_interval: Duration) -> AsyncLogger {
    let logger = AsyncLogger::new(log_file_path, flush_interval).await;
    let mut global_logger = LOGGER.lock().await;
    *global_logger = Some(logger.clone());
    logger
}

/// Log using LogLevel::Debug.
pub async fn log_debug(message: String) {
    if let Some(logger) = LOGGER.lock().await.as_ref() {
        logger.debug(message).await;
    }
}

/// Log using LogLevel::Info.
pub async fn log_info(message: String) {
    if let Some(logger) = LOGGER.lock().await.as_ref() {
        logger.info(message).await;
    }
}

/// Log using LogLevel::Warning.
pub async fn log_warning(message: String) {
    if let Some(logger) = LOGGER.lock().await.as_ref() {
        logger.warning(message).await;
    }
}

/// Log using LogLevel::Error.
pub async fn log_error(message: String) {
    if let Some(logger) = LOGGER.lock().await.as_ref() {
        logger.error(message).await;
    }
}

pub async fn get_logger_logs() -> Option<Arc<Mutex<Vec<LogEntry>>>> {
    let logger = LOGGER.lock().await;
    logger.as_ref().map(|logger| logger.get_logs_arc())
}
