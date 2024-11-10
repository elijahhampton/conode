use libp2p::{core::transport::ListenerId, Multiaddr};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub enum ListenerStatus {
    Active,                      // Listener is operational
    Expired,                     // Address is no longer valid
    Closed(ListenerCloseReason), // Listener has been closed
    Error(String),               // Listener encountered an error
}

#[derive(Clone, Debug)]
pub enum ListenerCloseReason {
    Done,          // Normal completion
    Error(String), // Error occurred
    Aborted,       // Operation aborted
    UserInitiated, // User requested closure
}
#[derive(Error, Debug)]
pub enum NetworkHandlerError {
    #[error("Listener {0} not found")]
    ListenerNotFound(ListenerId),
    #[error("Address {1} not found for listener {0}")]
    AddressNotFound(ListenerId, Multiaddr),
    #[error("Failed to acquire lock: {0}")]
    LockError(String),
    #[error("Invalid state transition from {0:?} to {1:?}")]
    InvalidStateTransition(ListenerStatus, ListenerStatus),
}

#[derive(Clone, Debug)]
pub struct KadPeerInfo {
    pub addresses: Vec<Multiaddr>,     // Known addresses for the peer
    pub last_seen: std::time::Instant, // Last contact timestamp
    pub is_new_peer: bool,             // Whether this is a newly discovered peer
}

pub struct NetworkConnectionHandler {
    pub listener_statuses: Arc<Mutex<HashMap<ListenerId, Vec<(Multiaddr, ListenerStatus)>>>>,
}

impl NetworkConnectionHandler {
    /// Creates a new `NetworkConnectionHandler` with an empty status map.
    ///
    /// Returns a new instance ready to track listener statuses and addresses.
    pub fn new() -> Self {
        Self {
            listener_statuses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registers a new listening address for a given listener ID.
    ///
    /// This method adds a new address to the specified listener with an initial status of `Active`.
    /// If the listener doesn't exist, it creates a new entry in the status map.
    ///
    /// # Arguments
    /// * `listener_id` - The ID of the listener to add the address to
    /// * `addr` - The multiaddress to register
    pub async fn new_listen_addr(&self, listener_id: ListenerId, addr: Multiaddr) {
        let mut statuses = self.listener_statuses.lock().await;

        // Get existing vec or create new one
        let addresses = statuses.entry(listener_id).or_insert_with(Vec::new);

        // Add the new address with Active status
        addresses.push((addr, ListenerStatus::Active));
    }

    /// Closes all addresses associated with a listener and updates their status with the provided reason.
    ///
    /// # Arguments
    /// * `listener_id` - The ID of the listener to close
    /// * `reason` - The reason for closing the listener
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful
    /// * `Err(NetworkHandlerError::ListenerNotFound)` if the listener doesn't exist
    pub async fn close_listener(
        &self,
        listener_id: ListenerId,
        reason: ListenerCloseReason,
    ) -> Result<(), NetworkHandlerError> {
        let mut statuses = self.listener_statuses.lock().await;

        let addresses = statuses
            .get_mut(&listener_id)
            .ok_or(NetworkHandlerError::ListenerNotFound(listener_id))?;

        // Update all addresses for this listener with the close reason
        for (_, status) in addresses.iter_mut() {
            *status = ListenerStatus::Closed(reason.clone());
        }

        Ok(())
    }

    /// Sets an error status for all addresses of a specified listener.
    ///
    /// # Arguments
    /// * `listener_id` - The ID of the listener to update
    /// * `error` - The error message to set (implements Display)
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful
    /// * `Err(NetworkHandlerError::ListenerNotFound)` if the listener doesn't exist
    pub async fn set_listener_error(
        &self,
        listener_id: ListenerId,
        error: impl std::fmt::Display,
    ) -> Result<(), NetworkHandlerError> {
        let mut statuses = self.listener_statuses.lock().await;

        let addresses = statuses
            .get_mut(&listener_id)
            .ok_or(NetworkHandlerError::ListenerNotFound(listener_id))?;

        // Set error status for all addresses of this listener
        for (_, status) in addresses.iter_mut() {
            *status = ListenerStatus::Error(error.to_string());
        }

        Ok(())
    }

    /// Marks a specific address for a listener as expired.
    ///
    /// If either the listener or address doesn't exist, this operation silently does nothing.
    ///
    /// # Arguments
    /// * `listener_id` - The ID of the listener containing the address
    /// * `addr` - The address to mark as expired
    pub async fn expire_address(&self, listener_id: ListenerId, addr: &Multiaddr) {
        if let Some(addresses) = self.listener_statuses.lock().await.get_mut(&listener_id) {
            for (existing_addr, status) in addresses.iter_mut() {
                if existing_addr == addr {
                    *status = ListenerStatus::Expired;
                }
            }
        }
    }

    /// Removes a listener and all its associated addresses from the status map.
    ///
    /// If the listener doesn't exist, this operation silently does nothing.
    ///
    /// # Arguments
    /// * `listener_id` - The ID of the listener to remove
    pub async fn remove_listener(&self, listener_id: ListenerId) {
        self.listener_statuses.lock().await.remove(&listener_id);
    }

    /// Retrieves all active addresses for a given listener.
    ///
    /// Returns only addresses that have an Active status. If the listener doesn't exist
    /// or has no active addresses, returns an empty vector.
    ///
    /// # Arguments
    /// * `listener_id` - The ID of the listener to query
    ///
    /// # Returns
    /// A vector of active Multiaddresses for the specified listener
    pub async fn get_active_addresses(&self, listener_id: ListenerId) -> Vec<Multiaddr> {
        self.listener_statuses
            .lock()
            .await
            .get(&listener_id)
            .map(|addresses| {
                addresses
                    .iter()
                    .filter(|(_, status)| matches!(status, ListenerStatus::Active))
                    .map(|(addr, _)| addr.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clears all internal state of the connection handler.
    /// This includes removing all listener statuses and their associated addresses.
    ///
    /// This method is typically used during shutdown to ensure clean cleanup of resources.
    pub async fn clear_state(&self) {
        // Acquire lock and clear the entire HashMap of listener statuses
        let mut statuses = self.listener_statuses.lock().await;

        // First set all active listeners to closed state for clean shutdown
        for (_, addresses) in statuses.iter_mut() {
            for (_, status) in addresses.iter_mut() {
                if matches!(status, ListenerStatus::Active) {
                    *status = ListenerStatus::Closed(ListenerCloseReason::UserInitiated);
                }
            }
        }

        // Then clear all entries
        statuses.clear();
    }
}
