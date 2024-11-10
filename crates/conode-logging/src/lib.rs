//! conode-logging crate
//!
//! This crate handles the logging functionality for the Conode project.
//! The crate providers an AsyncLogger that exposes a mpsc::Sender for LogEntries,
//! a thread and ownership safe Vector of LogEntries and a broadcast mpsc::sender for real
//! time updates.

pub mod logger;
