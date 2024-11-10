use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkManagerError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rocksdb::Error),
    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Job not found: {0}")]
    JobNotFound(String),
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
