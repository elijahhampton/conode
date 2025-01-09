use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum WorkManagerError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Serialization error: {0}")]
    SerdeError(String),
    #[error("Job not found: {0}")]
    JobNotFound(String),
    #[error("internal error, {0}")]
    Internal(String),
}

/// Errors related to critical problems dealing with storage
/// including bincode serialization and deserialization and
/// rocksdb.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Column family not found: {0}")]
    ColumnFamilyNotFound(&'static str),

    #[error("Failed to serialize data: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Database error: {0}")]
    Database(#[from] rocksdb::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error("internal contract class error")]
    Internal,
    #[error(
        "error processing event at block number {block_number} from address {address}: {context}"
    )]
    Event {
        block_number: u64,
        address: Felt,
        context: String,
    },
}

impl SyncError {}
