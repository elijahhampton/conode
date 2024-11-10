use anyhow::Result;
use backoff::{backoff::Backoff, ExponentialBackoff};
use conode_logging::logger::log_warning;
use conode_types::chain::ChainConfig;
use starknet::{
    core::{
        types::{BlockId, EmittedEvent, EventFilter, Felt},
        utils::get_selector_from_name,
    },
    providers::Provider,
};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use super::chain::ChainManager;
use crate::manager::storage::StorageManager;
use anyhow::anyhow;

/// SyncManager is responsible for syncing data retrieved from chain events
/// into the in-memory database. It handles blockchain synchronization and event processing.
pub struct SyncManager {
    /// Atomic flag indicating if a sync operation is in progress
    is_syncing: AtomicBool,
    /// Manager for chain operations
    chain_operator: ChainManager,
    /// Manager for persistent storage operations
    storage_manager: StorageManager,
    // Parameters related to the blockchain
    config: ChainConfig,
    /// Number of blocks to process in each sync chunk
    chunk_size: u64,
    // TODO: add_mpsc_channel_progress_network_events
    // event_sender: mpsc::Sender<NetworkEvent>,
}

impl SyncManager {
    /// Initial delay between retry attempts
    const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
    /// Maximum delay between retry attempts
    const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

    /// Creates a new SyncManager instance.
    ///
    /// # Arguments
    /// * `storage_manager` - Manager for persistent storage operations
    /// * `chain_operator` - Manager for chain operations
    ///
    /// # Returns
    /// * New SyncManager instance
    pub fn new(
        storage_manager: StorageManager,
        chain_operator: ChainManager,
        config: ChainConfig, // TODO: add_mpsc_channel_progress_network_events
                             //event_sender: mpsc::Sender<NetworkEvent>
    ) -> Self {
        Self {
            is_syncing: AtomicBool::new(false),
            chain_operator,
            storage_manager,
            chunk_size: 100,
            config,
            // TODO: add_mpsc_channel_progress_network_events
            //  event_sender,
        }
    }

    /// Synchronizes local state with the latest blockchain state.
    /// Uses exponential backoff for retry attempts.
    ///
    /// # Returns
    /// * Result indicating success or error
    pub async fn sync_to_latest_block(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self
            .is_syncing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return Ok(());
        }

        let mut backoff = ExponentialBackoff {
            initial_interval: Self::INITIAL_RETRY_DELAY,
            max_interval: Self::MAX_RETRY_DELAY,
            max_elapsed_time: Some(Duration::from_secs(120)),
            ..Default::default()
        };

        let _ = loop {
            match self.attempt_sync().await {
                Ok(_) => break Ok(()),
                Err(e) => {
                    if let Some(retry_delay) = backoff.next_backoff() {
                        log_warning(format!(
                            "Sync failed with error: {}. Retrying in {:?}...",
                            e, retry_delay
                        ))
                        .await;
                        sleep(retry_delay).await;
                        continue;
                    } else {
                        break Err(e);
                    }
                }
            }
        };

        self.is_syncing.store(false, Ordering::SeqCst);

        // TODO: add_mpsc_channel_progress_network_events
        // if let Err(e) = &result {
        //     self.event_sender.send(NetworkEvent::SyncFailed(e.to_string())).await?;
        // } else {
        //     self.event_sender.send(NetworkEvent::SyncCompleted).await?;
        // }

        Ok(())
    }

    /// Attempts to sync the local state with the blockchain.
    /// Processes blocks in chunks to avoid memory pressure.
    ///
    /// # Returns
    /// * Result indicating success or error
    async fn attempt_sync(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let latest_block = self
            .chain_operator
            .starknet_provider()
            .provider()
            .block_number()
            .await?;

        let last_synced = self.storage_manager.get_synced_block();

        if latest_block <= last_synced {
            return Ok(());
        }

        // Calculate chunks
        let blocks_to_sync = latest_block - last_synced;
        let chunks = (blocks_to_sync as f64 / self.chunk_size as f64).ceil() as u64;

        // TODO: add_mpsc_channel_progress_network_events
        // self.event_sender.send(NetworkEvent::SyncStarted {
        //     from_block: last_synced,
        //     to_block: latest_block
        // }).await?;

        // Sync in chunks
        for chunk in 0..chunks {
            let start_block = last_synced + (chunk * self.chunk_size) + 1;
            let end_block = std::cmp::min(start_block + self.chunk_size - 1, latest_block);

            // TODO: add_mpsc_channel_progress_network_events
            // self.event_sender.send(NetworkEvent::SyncProgress {
            //     current_block: start_block,
            //     target_block: latest_block,
            //     percentage: ((chunk as f64 + 1.0) / chunks as f64 * 100.0) as u8
            // }).await?;

            self.process_block_range(start_block, end_block).await?;
        }
        Ok(())
    }

    /// Processes a range of blocks, extracting and handling relevant events.
    ///
    /// # Arguments
    /// * `start_block` - First block number to process
    /// * `end_block` - Last block number to process
    ///
    /// # Returns
    /// * Result indicating success or error
    async fn process_block_range(
        &mut self,
        start_block: u64,
        end_block: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conode_contract_address = Felt::from_hex(&self.config.conode_contract).unwrap();
        for block_num in start_block..=end_block {
            let events = self
                .chain_operator
                .starknet_provider()
                .provider()
                .get_events(
                    EventFilter {
                        from_block: Some(BlockId::Number(block_num)),
                        to_block: Some(BlockId::Number(block_num)),
                        address: Some(conode_contract_address),
                        keys: None,
                    },
                    None,
                    self.chunk_size,
                )
                .await?;

            for event in events.events {
                if let Err(e) = self.process_event(event).await {
                    log_warning(format!(
                        "Failed to process event in block {}: {}",
                        block_num, e
                    ))
                    .await;
                }
            }

            let _ = self.storage_manager.store_synced_block(block_num);
        }

        Ok(())
    }

    /// Processes a single emitted event from the blockchain.
    /// Handles different event types including work submissions, registrations,
    /// and solution verifications.
    ///
    /// # Arguments
    /// * `event` - The blockchain event to process
    ///
    /// # Returns
    /// * Result indicating success or error
    async fn process_event(&self, event: EmittedEvent) -> Result<()> {
        let selector_work_submission: Felt =
            get_selector_from_name("WorkSubmission").unwrap_or(Felt::ZERO);

        match event.keys.get(0) {
            Some(key) => {
                if *key == selector_work_submission {
                    let work_id = event
                        .data
                        .get(0)
                        .ok_or_else(|| anyhow!("Missing work ID in WorkSubmission event"))?;
                    let chain_hash = event
                        .data
                        .get(1)
                        .ok_or_else(|| anyhow!("Missing chain hash in WorkSubmission event"))?;

                    let work_id_str = work_id.to_string();
                    let work_id_u128 = work_id_str.parse::<u128>().unwrap();
                    let uuid = Uuid::from_u128(work_id_u128);

                    // Store the submission
                    let _ = self
                        .storage_manager
                        .update_work_submission_hash(uuid.to_string(), chain_hash.to_owned());
                }
            }
            None => println!("Event without keys: {:?}", event),
        }

        Ok(())
    }

    /// Sets the number of blocks to process in each sync chunk.
    ///
    /// # Arguments
    /// * `size` - New chunk size value
    pub fn set_chunk_size(&mut self, size: u64) {
        self.chunk_size = size;
    }

    /// Checks if a sync operation is currently in progress.
    ///
    /// # Returns
    /// * `bool` - True if syncing is in progress
    pub fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::SeqCst)
    }
}
