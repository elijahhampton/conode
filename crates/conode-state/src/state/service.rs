use crate::sync::state::SyncState;
use conode_storage::{error::SyncError, manager::chain::ChainContext};
use conode_types::sync::SyncEvent;
use std::{error::Error, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct StateService {
    pub sync_state: Arc<RwLock<SyncState>>,
    pub event_rcv: tokio::sync::watch::Receiver<SyncEvent>,
    pub context: Arc<RwLock<ChainContext>>,
}

impl StateService {
    pub fn new(
        sync_state: Arc<RwLock<SyncState>>,
        context: Arc<RwLock<ChainContext>>,
        event_rcv: tokio::sync::watch::Receiver<SyncEvent>,
    ) -> Self {
        Self {
            sync_state,
            context,
            event_rcv,
        }
    }
    // Get a receiver to monitor sync events
    pub fn subscribe_to_events(&self) -> tokio::sync::watch::Receiver<SyncEvent> {
        self.event_rcv.clone()
    }

    // Initiates a manual sync operation
    pub async fn start_sync(&self) {
        // Use try_read instead of read to avoid blocking
        let is_syncing = match self.sync_state.try_read() {
            Ok(state) => state.is_syncing(),
            Err(_) => {
                warn!("Sync state lock contention, skipping sync");
                return;
            }
        };

        if is_syncing {
            info!("Sync already in progress, skipping");
            return;
        }

        // Use a timeout for sync operations
        match tokio::time::timeout(
            Duration::from_secs(300), // 5 minute timeout
            self.spawn_sync_task()
        ).await {
            Ok(result) => match result {
                Ok(_) => info!("Sync completed successfully"),
                Err(e) => error!("Sync failed: {}", e),
            },
            Err(_) => error!("Sync operation timed out"),
        }
    }

    //  Spawns the actual sync task
    async fn spawn_sync_task(&self) -> Result<(), SyncError> {
        let sync_state = self.sync_state.clone();
        
        // Use a separate channel for sync status updates
        let (status_tx, mut status_rx) = tokio::sync::mpsc::channel(10);
        
        let sync_handle = tokio::spawn(async move {
            let mut state = sync_state.write().await;

            if let Err(e) = state.sync_to_latest_block().await {
                error!("Sync failed: {}", e);
                let _ = status_tx.send(Err(e)).await;
                return Err(SyncError::Internal);
            }

            let _ = status_tx.send(Ok(())).await;
            Ok(())
        });

        // Wait for either completion or timeout
        tokio::select! {
            result = sync_handle => {
                match result {
                    Ok(inner_result) => inner_result,
                    Err(join_err) => Err(SyncError::Internal)
                };
            }
            Some(status) = status_rx.recv() => {
                return status;
            }
            else => {
                return Err(SyncError::Internal);
            }
        }

        Ok(())
    }

    // Spawns a task to monitor sync events
    async fn spawn_event_monitor(&self) {
        let mut event_rcv = self.event_rcv.clone();

        tokio::spawn(async move {
            while event_rcv.changed().await.is_ok() {
                let event = *event_rcv.borrow();
                match event {
                    SyncEvent::SyncStarted { from, to } => {
                        debug!("Sync started - from block {} to {}", from, to);
                    }
                    SyncEvent::SyncProgress {
                        current,
                        target,
                        percentage,
                    } => {
                        debug!(
                            "Sync progress: {:.2}% - block {} of {}",
                            percentage, current, target
                        );
                    }
                    SyncEvent::SyncFailed => {
                        
                    }
                    SyncEvent::SyncCompleted => {
                        
                 
                    }
                    SyncEvent::SyncAhead => {

                    },
                }
            }
        })
        .await;
    }

    // Checks if sync is currently running
    pub async fn is_syncing(&self) -> bool {
        let state = self.sync_state.read().await;
        state.is_syncing()
    }
}
