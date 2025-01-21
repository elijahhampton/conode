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

    pub fn subscribe_to_events(&self) -> tokio::sync::watch::Receiver<SyncEvent> {
        self.event_rcv.clone()
    }

    pub async fn start_sync(&self) {
        let is_syncing = match self.sync_state.try_read() {
            Ok(state) => state.is_syncing(),
            Err(_) => false
        };

        if !is_syncing {
            let _ = tokio::time::timeout(
                Duration::from_secs(300), 
                self.spawn_sync_task(),
            )
            .await;
        }
    }

    async fn spawn_sync_task(&self) -> Result<(), SyncError> {
        let sync_state = self.sync_state.clone();
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

 

    // Checks if sync is currently running
    pub async fn is_syncing(&self) -> bool {
        let state = self.sync_state.read().await;
        state.is_syncing()
    }
}
