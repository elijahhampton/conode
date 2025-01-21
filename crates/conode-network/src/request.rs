use crate::behaviour::DistributedBehaviour;
use crate::handle::NetworkStorage;
use crate::network::NetworkError;
use conode_storage::disk_db::DiskDb;
use conode_storage::manager::storage::InMemoryDb;
use conode_storage::traits::Storage;
use conode_storage::traits::StorageDefault;
use conode_types::negotiation::ProposalStatus;
use conode_types::negotiation::{Negotiation, NegotiationRequest};
use conode_types::sync::SyncEvent;
use libp2p::request_response::{Event, InboundRequestId, OutboundRequestId};
use libp2p::{PeerId, Swarm};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{sleep, Duration};

use std::io;
// The main event enum for the request/response protocol
#[derive(Debug)]
pub enum RequestResponseEvent {
    // Sent when an outbound request fails
    OutboundFailure {
        peer: PeerId,
        request_id: OutboundRequestId,
        error: RequestResponseError,
    },
    // Sent when an inbound request fails
    InboundFailure {
        peer: PeerId,
        request_id: InboundRequestId,
        error: RequestResponseError,
    },
    // Sent when we've successfully sent a response
    ResponseSent {
        peer: PeerId,
        request_id: InboundRequestId,
    },
    // Sent when we receive a response to our request
    Response {
        peer: PeerId,
        request_id: OutboundRequestId,
        response: Vec<u8>,
    },
    // Sent when we receive a new inbound request
    Message {
        peer: PeerId,
        request_id: InboundRequestId,
        request: Vec<u8>,
        channel: ResponseChannel,
    },
}
// Channel for sending responses back to inbound requests
#[derive(Debug)]
pub struct ResponseChannel {
    peer: PeerId,
    request_id: InboundRequestId,
    // Additional fields for the actual response channel implementation
}

// The different types of errors that can occur
#[derive(Debug)]
pub enum RequestResponseError {
    // The connection to the peer was closed
    ConnectionClosed,
    // The request timed out
    Timeout,
    // Failed to dial the peer
    DialFailure,
    // The remote doesn't support our protocols
    UnsupportedProtocols,
    // IO error occurred during read/write
    IoError(io::Error),
    // The request was cancelled
    Cancelled,
    // The remote is busy and cannot handle the request
    RemoteBusy,
    // The remote rejected our request
    Rejected,
    // Maximum number of concurrent outbound requests reached
    MaxOutboundRequestsPerPeer,
}

impl From<io::Error> for RequestResponseError {
    fn from(error: io::Error) -> Self {
        RequestResponseError::IoError(error)
    }
}

// Represents the state of an outbound request
#[derive(Serialize, Deserialize, Clone, Debug)]
enum RequestState {
    Pending,
    InProgress {
        attempts: u32,
        last_attempt: SystemTime,
    },
    Failed {
        reason: String,
    },
    Completed,
}

// Represents a pending network request
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PendingRequest {
    request_id: String,
    recipient: PeerId,
    negotiation: Negotiation,
    state: RequestState,
    created_at: SystemTime,
}

unsafe impl Send for PendingRequest {}
unsafe impl Sync for PendingRequest {}

// Configuration for retry behavior
#[derive(Clone)]
pub struct RetryConfig {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    backoff_factor: f32,
}

#[derive(Serialize, Deserialize)]
struct PeerMetrics {
    successful_requests: u64,
    failed_requests: u64,
    last_updated: SystemTime,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(300),
            backoff_factor: 2.0,
        }
    }
}

pub struct RequestHandler {
    swarm: Arc<Mutex<Swarm<DistributedBehaviour>>>,
    local_peer_id: PeerId,
    mem_db: Arc<Mutex<InMemoryDb>>,
    persistent_db: DiskDb,
    retry_config: RetryConfig,
    request_handlers: Arc<
        Mutex<
            HashMap<String, oneshot::Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
        >,
    >,
}

impl RequestHandler {
    pub fn new(
        swarm: Arc<Mutex<Swarm<DistributedBehaviour>>>,
        local_peer_id: PeerId,
        mem_db: Arc<Mutex<InMemoryDb>>,
        retry_config: RetryConfig,
    ) -> Self {
        Self {
            swarm,
            local_peer_id,
            mem_db,
            persistent_db: DiskDb::default(),
            retry_config,
            request_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn store_pending_request(
        &self,
        request: &PendingRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let serialized = serde_json::to_string(request)?;
        let key = format!("pending_request:{}", request.request_id);

        self.persistent_db
            .put_key(&key.as_bytes(), serialized.as_bytes(), None);
        Ok(())
    }

    fn update_request_state(
        &self,
        request_id: &str,
        new_state: RequestState,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("pending_request:{}", request_id);
        let existing_data = self.persistent_db.get_key(&key.as_bytes(), None)?.unwrap();

        let mut request: PendingRequest = serde_json::from_slice(&existing_data)?;
        request.state = new_state;

        let serialized = serde_json::to_string(&request)?;
        self.persistent_db
            .put_key(&key.as_bytes(), serialized.as_bytes(), None)?;

        Ok(())
    }

    async fn cleanup_old_requests(&self) {
        let cleanup_threshold = SystemTime::now()
            .checked_sub(Duration::from_secs(24 * 60 * 60))
            .unwrap();

        let mut batch_ops = Vec::new();

        let data = self
            .persistent_db
            .get_key(b"all_requests", None)
            .unwrap()
            .unwrap();
        // Collect keys to delete

        if let Ok(requests) = serde_json::from_slice::<Vec<PendingRequest>>(&data) {
            for request in requests {
                match request.state {
                    RequestState::Completed | RequestState::Failed { .. }
                        if request.created_at < cleanup_threshold =>
                    {
                        let key = format!("pending_request:{}", request.request_id);
                        batch_ops.push((key.into_bytes(), None)); // None means delete
                    }
                    _ => continue,
                }
            }
        }

        if let Err(e) = self.persistent_db.batch_write(None, batch_ops) {
            eprintln!("Failed to cleanup old requests: {}", e);
        }
    }

    async fn handle_request_response_event(
        &mut self,
        event: RequestResponseEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event {
            RequestResponseEvent::OutboundFailure {
                peer,
                request_id,
                error,
            } => {
                // Look up the original request in RocksDB using a composite key
                let key = format!("outbound_request:{}:{:?}", peer, request_id);
                let request_data = self.persistent_db.get_key(&key.as_bytes(), None)?.unwrap();

                let pending_request: PendingRequest = serde_json::from_slice(&request_data)?;

                // Analyze the error type to determine if retry is appropriate
                match error {
                    RequestResponseError::ConnectionClosed | RequestResponseError::Timeout => {
                        // These are retriable errors
                        self.handle_retriable_failure(&pending_request).await?;
                    }
                    RequestResponseError::DialFailure
                    | RequestResponseError::UnsupportedProtocols => {
                        // These are permanent failures
                        self.handle_permanent_failure(
                            &pending_request,
                            format!("Permanent failure: {:?}", error),
                        )
                        .await?;
                    }
                    RequestResponseError::IoError(error) => todo!(),
                    RequestResponseError::Cancelled => todo!(),
                    RequestResponseError::RemoteBusy => todo!(),
                    RequestResponseError::Rejected => todo!(),
                    RequestResponseError::MaxOutboundRequestsPerPeer => todo!(),
                }
            }

            RequestResponseEvent::InboundFailure {
                peer,
                request_id,
                error,
            } => {
                // Log inbound failures for monitoring
                log::warn!(
                    "Inbound failure from peer {}: {:?} (request_id: {:?})",
                    peer,
                    error,
                    request_id
                );

                // Update peer reliability metrics
                self.update_peer_metrics(&peer, false).await?;
            }

            RequestResponseEvent::ResponseSent { peer, request_id } => {
                // Track successful response for peer reliability
                self.update_peer_metrics(&peer, true).await?;

                // Clean up any pending request state
                let key = format!("outbound_request:{}:{:?}", peer, request_id);
                let request_data = self.persistent_db.get_key(&key.as_bytes(), None)?.unwrap();

                let mut pending_request: PendingRequest = serde_json::from_slice(&request_data)?;
                pending_request.state = RequestState::Completed;

                // Update final state
                self.store_pending_request(&pending_request)?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_retriable_failure(
        &mut self,
        request: &PendingRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &request.state {
            RequestState::InProgress {
                attempts,
                last_attempt,
            } => {
                if *attempts < self.retry_config.max_attempts {
                    // Calculate next retry delay using exponential backoff
                    let delay = self.calculate_retry_delay(*attempts);

                    // Schedule retry
                    let request_clone = request.clone();
                    //   let self_clone = Arc::new(self);

                    self.send_request_with_retry(&request_clone).await;
                } else {
                    self.handle_permanent_failure(
                        request,
                        "Max retry attempts exceeded".to_string(),
                    )
                    .await?;
                }
            }
            _ => {
                // Request is not in progress - possibly already handled
                log::warn!(
                    "Received failure for request {} in unexpected state: {:?}",
                    request.request_id,
                    request.state
                );
            }
        }
        Ok(())
    }

    async fn handle_permanent_failure(
        &mut self,
        request: &PendingRequest,
        reason: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Update request state to failed
        let mut updated_request = request.clone();
        updated_request.state = RequestState::Failed {
            reason: reason.clone(),
        };
        self.store_pending_request(&updated_request);

        // Notify any waiting handlers
        if let Some(sender) = self
            .request_handlers
            .lock()
            .await
            .remove(&request.request_id)
        {
            let _ = sender.send(Err(reason.into()));
        }

        // Update peer reliability metrics
        self.update_peer_metrics(&request.recipient, false).await?;

        Ok(())
    }

    async fn update_peer_metrics(
        &mut self,
        peer: &PeerId,
        success: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("peer_metrics:{}", peer);
        let val_or_none = self.persistent_db.get_key(&key.as_bytes(), None)?;

        let metrics = if val_or_none.is_some() {
            let mut metrics: PeerMetrics = serde_json::from_slice(&val_or_none.unwrap())?;
            if success {
                metrics.successful_requests += 1;
            } else {
                metrics.failed_requests += 1;
            }
            metrics
        } else {
            PeerMetrics {
                successful_requests: if success { 1 } else { 0 },
                failed_requests: if success { 0 } else { 1 },
                last_updated: SystemTime::now(),
            }
        };

        // Store updated metrics
        let serialized = serde_json::to_string(&metrics)?;
        self.persistent_db
            .put_key(&key.as_bytes(), serialized.as_bytes(), None)?;

        Ok(())
    }

    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.retry_config.initial_delay;
        let backoff = (self.retry_config.backoff_factor as u32).pow(attempt);
        std::cmp::min(base_delay * backoff, self.retry_config.max_delay)
    }

    async fn send_single_request(
        &mut self,
        recipient: &PeerId,
        request: NegotiationRequest,
    ) -> Result<OutboundRequestId, Box<dyn std::error::Error + Send + Sync>> {
        let outbound_req_id = self
            .swarm
            .lock()
            .await
            .behaviour_mut()
            .conode
            .negotiation_protocol
            .send_request(recipient, request);

        Ok(outbound_req_id)
    }

    async fn send_request_with_retry(
        &mut self,
        pending_request: &PendingRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut current_attempt = 0;
        let mut delay = self.retry_config.initial_delay;

        while current_attempt < self.retry_config.max_attempts {
            // Create the request
            let request = NegotiationRequest::NewProposal(
                self.local_peer_id.clone(),
                pending_request.negotiation.clone(),
            );

            // Update request state to InProgress
            self.update_request_state(
                &pending_request.request_id,
                RequestState::InProgress {
                    attempts: current_attempt + 1,
                    last_attempt: SystemTime::now(),
                },
            )?;

            // Set up response channel for this attempt
            let (tx, rx) = oneshot::channel();
            self.request_handlers
                .lock()
                .await
                .insert(pending_request.request_id.clone(), tx);

            // Send the request
            match self
                .send_single_request(&pending_request.recipient, request)
                .await
            {
                Ok(_) => {
                    // Wait for response or timeout
                    match tokio::time::timeout(
                        Duration::from_secs(30), // Configurable timeout
                        rx,
                    )
                    .await
                    {
                        Ok(Ok(_)) => {
                            // Success - update state and return
                            self.update_request_state(
                                &pending_request.request_id,
                                RequestState::Completed,
                            )?;
                            return Ok(());
                        }
                        Ok(Err(e)) => {
                            // Timeout or channel error - retry
                            current_attempt += 1;
                            if current_attempt == self.retry_config.max_attempts {
                                return Err(e.into());
                            }
                            sleep(delay).await;
                            delay = std::cmp::min(
                                delay.mul_f32(self.retry_config.backoff_factor),
                                self.retry_config.max_delay,
                            );
                        }
                        Err(e) => {
                            // Timeout or channel error - retry
                            current_attempt += 1;
                            if current_attempt == self.retry_config.max_attempts {
                                return Err(e.into());
                            }
                            sleep(delay).await;
                            delay = std::cmp::min(
                                delay.mul_f32(self.retry_config.backoff_factor),
                                self.retry_config.max_delay,
                            );
                        }
                    }
                }
                Err(e) => {
                    current_attempt += 1;
                    if current_attempt == self.retry_config.max_attempts {
                        self.update_request_state(
                            &pending_request.request_id,
                            RequestState::Failed {
                                reason: e.to_string(),
                            },
                        )?;
                        return Err(e);
                    }
                    sleep(delay).await;
                    delay = std::cmp::min(
                        delay.mul_f32(self.retry_config.backoff_factor),
                        self.retry_config.max_delay,
                    );
                }
            }
        }

        Err(NetworkError::MaxRetriesExceeded.into())
    }
}
