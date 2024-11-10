use conode_types::negotiation::Negotiation;
use rocksdb::{Direction, IteratorMode, DB};
use starknet::core::types::Felt;
use std::sync::Arc;
use std::{collections::HashMap, error::Error};

use crate::error::{StoreError, WorkManagerError};
use conode_logging::logger::log_info;
use conode_types::work::{ActiveWork, PendingItem, Work, WorkBroadcast};

/// Storage manager for handling persistent data storage using RocksDB.
/// Manages work items, proposals, and blockchain sync state.
#[derive(Debug, Clone)]
pub struct StorageManager {
    /// The underlying RocksDB instance
    db: Arc<DB>,
}

impl StorageManager {
    /// Creates a new StorageManager instance with the provided RocksDB database.
    ///
    /// # Arguments
    /// * `db` - Arc wrapped RocksDB instance
    ///
    /// # Returns
    /// * `Result<Self, WorkManagerError>` - New StorageManager instance or error
    pub fn new(db: Arc<DB>) -> Result<Self, WorkManagerError> {
        Ok(StorageManager { db })
    }

    /// Stores the latest synchronized block number.
    ///
    /// # Arguments
    /// * `block_number` - Block number to store
    ///
    /// # Returns
    /// * Result(()) if put() returns successful or StoreError if a storage error occurs.
    pub fn store_synced_block(&self, block_number: u64) -> Result<(), StoreError> {
        self.db
            .put("synced_block_number", block_number.to_string().as_bytes())
            .map_err(|e| StoreError::Database(e))
    }

    /// Retrieves the last synchronized block number.
    ///
    /// # Returns
    /// * `u64` - Last synced block number, or 0 if none found
    pub fn get_synced_block(&self) -> u64 {
        match self.db.get("synced_block_number") {
            Ok(Some(value)) => {
                // Parse block number from stored value
                String::from_utf8(value).unwrap().parse().unwrap_or(0)
            }
            Ok(None) => 0, // Default to 0 if no value is found
            Err(_) => 0,   // Handle any error by defaulting to 0
        }
    }

    /// Adds a work broadcast that has been sent by this node.
    ///
    /// # Arguments
    /// * `work` - Work item to store
    pub fn add_broadcasted_work(&self, work: Work) -> Result<(), StoreError> {
        let cf_handle = self.db.cf_handle("broadcasted_work");

        if cf_handle.is_some() {
            let serialization_result = bincode::serialize(&work);
            match serialization_result {
                Ok(serialization_bytes) => {
                    let key = work.id.clone();
                    let _ = self
                        .db
                        .put_cf(&cf_handle.unwrap(), key, serialization_bytes);
                }
                Err(e) => return Err(StoreError::Serialization(e)),
            }
        }

        Ok(())
    }

    /// Stores a potential work item received from the network.
    ///
    /// # Arguments
    /// * `work` - Work broadcast to store
    ///
    /// # Returns
    /// * `Result<(), StoreError>` - Success or error storing work
    pub fn add_potential_work(&self, work: WorkBroadcast) -> Result<(), StoreError> {
        let id = work.work.id.clone();

        let pending_item = PendingItem {
            work: work,
            is_marked_for_save: false,
        };

        let cf_handle = self.db.cf_handle("gossiped_work");

        if cf_handle.is_some() {
            match bincode::serialize(&pending_item) {
                Ok(bytes) => {
                    let key = format!("{}", id);
                    self.db.put_cf(&cf_handle.unwrap(), key.as_bytes(), bytes)?;
                }
                Err(e) => return Err(StoreError::Serialization(e)),
            }
        }

        Ok(())
    }

    /// Retrieves all stored work broadcasts.
    ///
    /// # Returns
    /// * `Result<Vec<WorkBroadcast>, StoreError>` - List of work broadcasts or error
    pub fn get_work(&self) -> Result<Vec<WorkBroadcast>, StoreError> {
        let mut works = Vec::new();
        let cf_handle = self.db.cf_handle("gossiped_work");

        if cf_handle.is_some() {
            let iter = self
                .db
                .iterator_cf(&cf_handle.unwrap(), rocksdb::IteratorMode::Start);

            for item in iter {
                match item {
                    Ok((_key, value)) => {
                        let pending_item: PendingItem = bincode::deserialize(&value)
                            .map_err(|e| StoreError::Serialization(e))?;
                        works.push(pending_item.work);
                    }
                    Err(e) => return Err(StoreError::from(e)),
                }
            }

            return Ok(works);
        }

        Err(StoreError::ColumnFamilyNotFound("gossiped_work"))
    }

    /// Retrieves all active work items.
    ///
    /// # Returns
    /// * `Result<Vec<ActiveWork>, rocksdb::Error>` - List of active work or error
    pub fn get_active_work(&self) -> Result<Vec<ActiveWork>, rocksdb::Error> {
        let mut active_works = Vec::new();

        let db_iter = self.db.iterator_cf(
            &self.db.cf_handle("active_work").unwrap(),
            IteratorMode::End,
        );
        for item in db_iter {
            match item {
                Ok((_key, value)) => {
                    let work: ActiveWork = bincode::deserialize(&value).unwrap();
                    active_works.push(work);
                }
                Err(e) => return Err(e),
            }
        }

        Ok(active_works)
    }

    /// Retrieves an active work item by its ID.
    ///
    /// # Arguments
    /// * `work_id` - ID of the work to retrieve
    ///
    /// # Returns
    /// * `anyhow::Result<Option<ActiveWork>>` - Active work if found or None
    pub fn get_active_work_by_id(&self, work_id: String) -> anyhow::Result<Option<ActiveWork>> {
        let cf_handle = match self.db.cf_handle("active_work") {
            Some(cf) => cf,
            None => return Ok(None),
        };

        let bytes = match self.db.get_cf(&cf_handle, work_id) {
            Ok(Some(bytes)) => bytes,
            _ => return Ok(None),
        };

        match bincode::deserialize(&bytes) {
            Ok(work) => Ok(Some(work)),
            Err(_) => Ok(None),
        }
    }

    /// Stores an active work item.
    ///
    /// # Arguments
    /// * `work` - Active work to store
    ///
    /// # Returns
    /// * `Result<(), StoreError>` - Success or error
    pub fn store_active_work(&self, work: ActiveWork) -> Result<(), StoreError> {
        let cf_handle = self
            .db
            .cf_handle("active_work")
            .ok_or(StoreError::ColumnFamilyNotFound("active_work"))?;

        let key = work.work.id.clone();

        let serialized_work =
            bincode::serialize(&work).map_err(|e| StoreError::Serialization(e))?;

        self.db
            .put_cf(&cf_handle, key.as_bytes(), serialized_work)
            .map_err(|e| StoreError::Database(e))?;

        Ok(())
    }

    /// Saves or updates a negotiation proposal.
    ///
    /// # Arguments
    /// * `negotiation` - Negotiation to save or update
    ///
    /// # Returns
    /// * `Result<(), String>` - Success or error message
    pub async fn save_or_update_proposal(&self, negotiation: &Negotiation) -> Result<(), String> {
        let key = format!("proposals:{}", negotiation.id);

        let serialized_negotiation = bincode::serialize(negotiation);
        if let Ok(serialized_negotiation) = serialized_negotiation {
            match self.db.put(key.as_bytes(), serialized_negotiation) {
                Ok(_) => {
                    log_info(format!("Saved proposal with id {}", negotiation.id)).await;
                    Ok(())
                }
                Err(err) => {
                    log_info(format!(
                        "Failed to save proposal with id {}",
                        negotiation.id
                    ))
                    .await;
                    Err(err.into_string())
                }
            }
        } else {
            Err(serialized_negotiation
                .map_err(|e| e.to_string())
                .unwrap_err())
        }
    }

    /// Retrieves a proposal by its ID.
    ///
    /// # Arguments
    /// * `negotiation_id` - ID of the proposal to retrieve
    ///
    /// # Returns
    /// * `Result<Option<Negotiation>, Box<dyn Error>>` - Proposal if found or error
    pub fn get_proposal_by_id(
        &self,
        negotiation_id: &str,
    ) -> Result<Option<Negotiation>, Box<dyn Error>> {
        let prefix = "proposals:";
        let iter = self
            .db
            .iterator(IteratorMode::From(prefix.as_bytes(), Direction::Forward));

        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            // Only process keys with the proposals prefix
            if !key_str.starts_with(prefix) {
                break;
            }

            // Check if this key contains our negotiation_id
            if key_str.ends_with(negotiation_id) {
                return Ok(Some(bincode::deserialize(&value)?));
            }
        }

        Ok(None)
    }

    /// Retrieves all stored proposals.
    ///
    /// # Returns
    /// * `Result<Vec<Negotiation>, Box<dyn Error + Send + Sync>>` - List of proposals or error
    pub async fn get_all_proposals(
        &self,
    ) -> Result<Vec<Negotiation>, Box<dyn Error + Send + Sync>> {
        let mut proposals = Vec::new();
        let prefix = b"proposals:";

        let iter = self
            .db
            .iterator(IteratorMode::From(prefix, Direction::Forward));
        for item in iter {
            let (key, value) = item?;
            if !key.starts_with(prefix) {
                break;
            }

            match bincode::deserialize::<Negotiation>(&value) {
                Ok(proposal) => proposals.push(proposal),
                Err(e) => return Err(e),
            }
        }

        log_info(format!("Total proposals found: {}", proposals.len())).await;
        Ok(proposals)
    }

    /// Retrieves a proposal by ID.
    ///
    /// # Arguments
    /// * `negotiation_id` - ID of the proposal to retrieve
    ///
    /// # Returns
    /// * `Option<Negotiation>` - Proposal if found
    pub fn get_proposal(&self, negotiation_id: &str) -> Option<Negotiation> {
        let key = format!("proposals:{}", negotiation_id);

        // Get the bytes from db
        let bytes = match self.db.get(key.as_bytes()) {
            Ok(bytes) => bytes,
            Err(_e) => {
                // tracing::error!("Failed to get proposal from db: {}", e);
                return None;
            }
        };

        if bytes.is_none() {
            return None;
        }

        // Deserialize the bytes
        match bincode::deserialize(&bytes.unwrap()) {
            Ok(negotiation) => Some(negotiation),
            Err(_e) => {
                // tracing::error!("Failed to deserialize proposal: {}", e);
                None
            }
        }
    }

    /// Stores a solution for a work item.
    ///
    /// # Arguments
    /// * `work_id` - ID of the work
    /// * `solution` - Solution to store
    ///
    /// # Returns
    /// * `Result<(), Box<dyn Error>>` - Success or error
    pub fn store_work_solution(
        &self,
        work_id: String,
        solution: String,
    ) -> Result<(), Box<dyn Error>> {
        // Get handle for solutions column family
        let cf_handle = self
            .db
            .cf_handle("solutions")
            .ok_or("Column family 'solutions' not found")?;

        // Store the solution with work ID as key
        self.db.put_cf(&cf_handle, work_id, solution)?;

        Ok(())
    }

    /// Stores a salt value for a work solution.
    ///
    /// # Arguments
    /// * `work_id` - ID of the work
    /// * `salt` - Salt value to store
    ///
    /// # Returns
    /// * `Result<(), Box<dyn Error>>` - Success or error
    pub fn store_work_salt(&self, work_id: String, salt: String) -> Result<(), Box<dyn Error>> {
        // Get handle for salts column family
        let cf_handle = self
            .db
            .cf_handle("solution_salts")
            .ok_or("Column family 'solution_salts' not found")?;

        // Store the salt with work ID as key
        self.db.put_cf(&cf_handle, work_id.as_bytes(), salt)?;

        Ok(())
    }

    /// Updates the submission hash for a work item.
    ///
    /// # Arguments
    /// * `work_id` - ID of the work
    /// * `submission_hash` - Hash of the submission
    ///
    /// # Returns
    /// * `Result<(), Box<dyn Error>>` - Success or error
    pub fn update_work_submission_hash(
        &self,
        work_id: String,
        submission_hash: Felt,
    ) -> Result<(), Box<dyn Error>> {
        // Use a dedicated column family for submissions
        let cf_handle = self
            .db
            .cf_handle("solution_hashes")
            .ok_or("Column family 'solution_hashes' not found")?;

        // Store the submission hash with the work ID as key
        self.db.put_cf(
            &cf_handle,
            work_id.as_bytes(),
            submission_hash.to_bytes_be().as_slice(),
        )?;

        Ok(())
    }

    /// Retrieves a work submission by ID.
    ///
    /// # Arguments
    /// * `work_id` - ID of the work
    ///
    /// # Returns
    /// * `Result<Option<String>, Box<dyn Error + Send + Sync>>` - Submission if found or error
    pub fn get_work_submission(
        &self,
        work_id: &str,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let cf_handle = self
            .db
            .cf_handle("solutions")
            .ok_or("Column family 'solutions' not found")?;

        match self.db.get_cf(&cf_handle, work_id.as_bytes())? {
            Some(bytes) => Ok(Some(bincode::deserialize(bytes.as_slice())?)),
            None => Ok(None),
        }
    }

    /// Retrieves submissions for multiple work items.
    ///
    /// # Arguments
    /// * `works` - List of active work items
    ///
    /// # Returns
    /// * `Result<HashMap<String, Option<String>>, Box<dyn Error + Send + Sync>>` - Map of work IDs to submissions
    pub fn get_work_submissions(
        &self,
        works: &Vec<ActiveWork>,
    ) -> Result<HashMap<String, Option<String>>, Box<dyn Error + Send + Sync>> {
        let mut submissions_map = HashMap::new();

        for work in works.iter() {
            let submission = self.get_work_submission(&work.work.id.clone())?;

            submissions_map.insert(work.work.id.to_owned(), submission);
        }

        Ok(submissions_map)
    }

    /// Retrieves a submission hash for a work item.
    ///
    /// # Arguments
    /// * `work_id` - ID of the work
    ///
    /// # Returns
    /// * `Result<Option<Felt>, Box<dyn Error + Send + Sync>>` - Submission hash if found or error
    pub fn get_work_submission_hash(&self, work_id: &str) -> Option<Felt> {
        let cf_handle = self.db.cf_handle("solution_hashes");
        if cf_handle.is_none() {
            return None;
        }

        match self.db.get_cf(&cf_handle.unwrap(), work_id.as_bytes()) {
            Ok(Some(bytes)) => {
                let bytes_deserialized = Felt::from_bytes_be_slice(bytes.as_slice());
                Some(bytes_deserialized)
            }
            Ok(None) => None,
            Err(_) => None,
        }
    }

    /// Retrieves submission hashes for multiple work items.
    ///
    /// # Arguments
    /// * `works` - List of active work items
    ///
    /// # Returns
    /// * `Result<HashMap<String, Option<Felt>>, Box<dyn Error + Send + Sync>>` - Map of work IDs to submission hashes
    pub fn get_work_submission_hashes(
        &self,
        works: &Vec<ActiveWork>,
    ) -> Option<HashMap<String, Option<Felt>>> {
        let mut submissions_map = HashMap::new();

        for work in works.iter() {
            let submission = self.get_work_submission_hash(&work.work.id.clone());

            if submission.is_some() {
                submissions_map.insert(work.work.id.to_owned(), submission);
            }
        }

        Some(submissions_map)
    }
}
