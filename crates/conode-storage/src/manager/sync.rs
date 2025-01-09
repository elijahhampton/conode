use anyhow::Result;
use backoff::{backoff::Backoff, ExponentialBackoff};
use conode_logging::logger::log_warning;
use conode_types::{
    chain::ChainConfig,
    sync::{SyncEvent, SyncRange},
};
use starknet::{
    core::{
        types::{BlockId, EmittedEvent, EventFilter, Felt},
        utils::get_selector_from_name,
    },
    providers::Provider,
};
use std::sync::{atomic::AtomicBool, Arc};
use std::{
    sync::atomic::Ordering,
};
use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
};
use tracing::info;
use uuid::Uuid;

use super::{chain::ChainContext, storage::InMemoryDb};
use crate::error::SyncError;

/// SyncManager is responsible for syncing data retrieved from chain events
/// into the in-memory database. It handles blockchain synchronization and event processing.
pub struct SyncState {
    /// Atomic flag indicating if a sync operation is in progress
    pub is_syncing: AtomicBool,
    /// Manager for chain operations
    chain_operator: Arc<Mutex<ChainContext>>,
    /// Manager for persistent storage operations
    storage_manager: Arc<Mutex<InMemoryDb>>,
    // Parameters related to the blockchain
    config: ChainConfig,
    /// Number of blocks to process in each sync chunk
    chunk_size: u64,
    /// An mpc channel receiver for NetworkEvents
    pub event_receiver: tokio::sync::watch::Receiver<SyncEvent>,
    /// An mpc channel sender for NetworkEvents
    event_sender: tokio::sync::watch::Sender<SyncEvent>,
}

impl SyncState {
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
        storage_manager: Arc<Mutex<InMemoryDb>>,
        chain_operator: Arc<Mutex<ChainContext>>,
        config: ChainConfig,
    ) -> Self {
        let (event_sender, event_receiver) = tokio::sync::watch::channel(SyncEvent::SyncCompleted);
        Self {
            is_syncing: AtomicBool::new(false),
            chain_operator,
            storage_manager,
            chunk_size: 1000,
            config,
            event_sender,
            event_receiver,
        }
    }

    /// Synchronizes local state with the latest blockchain state.
    /// Uses exponential backoff for retry attempts.
    ///
    /// # Returns
    /// * Result indicating success or error
    pub async fn sync_to_latest_block(&mut self) -> Result<(), SyncError> {
        println!("sync_to_latest_block");
        // if !self
        //     .is_syncing
        //     .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        //     .is_ok()
        // {
        //     info!("sync_toffffffffff_latest_block");
        //     return Ok(());
        // }

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
                        info!("{:?}", e);
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

    /// Attempts to sync blockchain events starting at the deployment block
    /// of the CoNode WorkCore smart contract.
    /// # Returns
    /// * Result indicating success or error
    async fn attempt_sync(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        
        let chain_tip = {
            self
            .chain_operator
            .lock()
            .await
            .starknet_provider()
            .provider()
            .block_number()
            .await?
        };
        
        let latest_synced_block_num = self.storage_manager.lock().await.latest_synced_block();
   
        if chain_tip <= latest_synced_block_num {
            return Ok(());
        }

        // Calculate blo
        let blocks_to_sync = chain_tip - latest_synced_block_num;
        let block_ranges = (blocks_to_sync as f64 / self.chunk_size as f64).ceil() as u64;

        self.event_sender
            .send(SyncEvent::SyncStarted {
                from: latest_synced_block_num,
                to: chain_tip,
            })
            .map_err(|e| SyncError::Internal)?;

        // Sync in chunks
        for range in 0..block_ranges {
            let current = latest_synced_block_num + (range * self.chunk_size) + 1;
            let target = std::cmp::min(current + self.chunk_size - 1, chain_tip);

            self.event_sender.send(SyncEvent::SyncProgress {
                current,
                target,
                percentage: ((range as f64 + 1.0) / block_ranges as f64 * 100.0),
            })?;

            self.process_block_range(current, target)
                .await
                .map_err(|_| SyncError::Internal)?;
      
        }

        self
            .storage_manager
            .lock()
            .await
            .update_latest_synced_block_num(chain_tip)?;

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
        &self,
        start: u64,
        end: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut event_batch = Vec::new();

        let conode_contract_address = Felt::from_hex(&self.config.conode_contract).unwrap();
        let chain_context = self.chain_operator.lock().await;

        let provider = chain_context.starknet_provider().provider();

        let events = provider
            .get_events(
                EventFilter {
                    from_block: Some(BlockId::Number(start)),
                    to_block: Some(BlockId::Number(end)),
                    address: Some(conode_contract_address),
                    keys: None,
                },
                None,
                self.chunk_size,
            )
            .await?;

        // We need to check for a continuation token returned by get_events() since the response
        // is paginated and continue adding each batch to our buffer for processing.
        event_batch.extend_from_slice(&events.events);

        let mut continuation_token = events.continuation_token;
        while continuation_token.is_some() {
            let events_with_continuation = provider
                .get_events(
                    EventFilter {
                        from_block: Some(BlockId::Number(start)),
                        to_block: Some(BlockId::Number(end)),
                        address: Some(conode_contract_address),
                        keys: None,
                    },
                    continuation_token,
                    self.chunk_size,
                )
                .await?;
            event_batch.extend_from_slice(&events_with_continuation.events);

            continuation_token = events_with_continuation.continuation_token;
        }

        Ok(self.process_event_batch(&event_batch).await?)
    }

    async fn process_event_batch(&self, events: &Vec<EmittedEvent>) -> Result<(), SyncError> {
     

        for event in events {
            self.process_event(&event).await?
        }

        Ok(())
    }

    /// Processes a single emitted event from the blockchain.
    /// Handles different event types including work submissions, registrations,
    /// and solution verifications.
    ///
    /// # Arguments
    /// * `event` - The blockchain [`EmittedEvent`] to process
    ///
    /// # Returns
    /// * Result(()) An empty result if successful
    /// * SyncError A [`SyncError`] if err
    async fn process_event(&self, event: &EmittedEvent) -> Result<(), SyncError> {
        let chain_context = self.chain_operator.lock().await;

        let selector_work_submission = chain_context
            .selector_as_felt("WorkSubmission")
            .unwrap_or(&Felt::ZERO);

        match event.keys.get(0) {
            Some(key) => {
                if *key == *selector_work_submission {
                    let work_id = event.data.get(0).ok_or_else(|| SyncError::Event {
                        block_number: event.block_number.unwrap(),
                        address: event.from_address,
                        context: "missing task_id key".to_string(),
                    })?;
                    let chain_hash = event.data.get(1).ok_or_else(|| SyncError::Event {
                        block_number: event.block_number.unwrap(),
                        address: event.from_address,
                        context: "missing chain hash".to_string(),
                    })?;

                    let uuid = Uuid::new_v4(); //FlattenedWork::task_id_felt_to_uuid(*work_id);

                    let storage = &*self.storage_manager.lock().await;
                    storage
                        .update_work_submission_hash(uuid.to_string(), chain_hash.to_owned())
                        .map_err(|_| SyncError::Event {
                            block_number: event.block_number.unwrap(),
                            address: event.from_address,
                            context: "updating work submission hash failed".to_string(),
                        })?;
                    //.map_err(|e| SyncError::Event { block_number: event.block_number.unwrap(), address: event.from_address, context: "updating work submission hash failed".to_string() })?;
                }
            }
            None => {
                return Err(SyncError::Event {
                    block_number: event.block_number.unwrap(),
                    address: event.from_address,
                    context: "abi contains no chain events".to_string(),
                })
            }
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
