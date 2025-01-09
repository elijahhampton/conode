

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum SyncState {
    Idle,
    Syncing,
    Error(String),
}

/// A sync range representing the full range of blocks that were covered
/// during the syncing process.
#[derive(Serialize, Deserialize)]
pub struct SyncRange {
    pub start: u64,
    pub end: u64,
}

unsafe impl Send for SyncRange {}
unsafe impl Sync for SyncRange {}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum SyncEvent {
    // Syncing failed
    SyncFailed,
    // Syncing completed
    SyncCompleted,
    // The start and target block of the syncing process
    SyncStarted {
        from: u64,
        to: u64,
    },
    // The current state of the syncing process
    SyncProgress {
        current: u64,
        target: u64,
        percentage: f64,
    },
    // The node is currently ahead of the block provider
    SyncAhead
}

unsafe impl Send for SyncEvent {}
unsafe impl Sync for SyncEvent {}
