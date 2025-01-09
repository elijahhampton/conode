use crate::network::Network;
use crate::protocol::discovery::traits::Seeker;
use crate::protocol::discovery::work_discovery_topic;
use crate::protocol::negotiation::traits::Negotiator;

use conode_storage::error::WorkManagerError;
use conode_types::peer::PeerInfo;

use conode_types::traits::libp2p::GossipsubNodeProvider;
use conode_types::work::FlattenedWork;
use conode_types::{
    negotiation::{Negotiation, NegotiationRequest, NegotiationResponse},
    work::WorkBroadcast,
};
use futures::StreamExt;

use libp2p::{
    core::transport::ListenerId,
    gossipsub::{Message as GossipsubMessage, Sha256Topic, SubscriptionError},
    request_response::{InboundRequestId, OutboundRequestId, ResponseChannel},
    Multiaddr, PeerId,
};
use libp2p_kad::RoutingUpdate;
use log::{error, warn};
use std::time::Duration;
use std::{error::Error, sync::Arc};
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot, Mutex,
};

use conode_storage::traits::{Storage, StorageDefault, StorageWithDefault};

pub trait NetworkStorage: Send + Sync + 'static {
    fn as_storage(&self) -> &dyn Storage;
}

impl Storage for Box<dyn NetworkStorage> {
    fn get_key(&self, key: &[u8], cf_name: Option<&str>) -> Result<Option<Vec<u8>>, WorkManagerError> {
        self.as_storage().get_key(key, cf_name)
    }

    fn put_key(&self, key: &[u8], value: &[u8], cf_name: Option<&str>) -> Result<(), WorkManagerError> {
        self.as_storage().put_key(key, value, cf_name)
    }

    fn delete_key(&self, key: &[u8], cf_name: Option<&str>) -> Result<(), WorkManagerError> {
        self.as_storage().delete_key(key, cf_name)
    }

    fn batch_write(&self, cf_name: Option<&str>, batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<(), WorkManagerError> {
        self.as_storage().batch_write(cf_name, batch)
    }
}

impl<T: Storage + Send + Sync + 'static> NetworkStorage for T {
    fn as_storage(&self) -> &dyn Storage {
        self
    }
}

/// An interface to the underlying CoNodeNetwork.
#[derive(Clone)]
pub struct NetworkHandle {
    /// The Arc'ed delegate that contains the state.
    inner: Arc<NetworkInner>,
}

/// NetworkInner contains the underlying network struct as well as a unbounder sender
/// that relays messages to the network for further handling.
struct NetworkInner {
    /// The network instance
    network: Arc<Mutex<Network>>,
    /// Channel to send messages to the network manager
    to_manager_tx: Sender<NetworkHandleMessage>,
}

#[derive(Debug)]
pub enum NetworkHandleMessage {
    /// Broadcast work to the network
    BroadcastWork(WorkBroadcast),
    /// Initiate negotiation with a peer
    InitiateNegotiation {
        recipient: PeerId,
        job_id: String,
        negotiation: Negotiation,
    },
    /// Request completion acknowledgement
    RequestCompletionAck {
        recipient: PeerId,
        negotiation: Negotiation,
    },
    /// Send completion confirmation request
    SendCompletionConfirmationRequest {
        recipient: PeerId,
        negotiation: String,
    },
    /// Send completion confirmation acknowledgement
    SendCompletionConfirmationAck {
        to: PeerId,
        work_id: String,
        solution_uri: String,
    },
    /// Handle negotiation request
    HandleNegotiationRequest {
        from: PeerId,
        request_id: InboundRequestId,
        request: NegotiationRequest,
        channel: ResponseChannel<NegotiationResponse>,
    },
    /// Handle negotiation response
    HandleNegotiationResponse {
        request_id: OutboundRequestId,
        response: NegotiationResponse,
    },
    /// Handle gossipsub message
    HandleGossipsubMessage(GossipsubMessage),
    /// Subscribe to a topic
    Subscribe(String),
    /// Unsubscribe from a topic
    Unsubscribe(String),
    /// Add new listener
    AddListener(ListenerId, Multiaddr),
    /// Start listening on address
    StartListening(Multiaddr),
    /// Join gossip protocol
    JoinGossipProtocol,
    /// Shutdown the network
    Shutdown(oneshot::Sender<()>),
}

impl NetworkHandle {
    /// Creates a new instance of NetworkHandle
    pub fn new(network: Arc<Mutex<Network>>) -> Self {
        let (to_manager_tx, mut to_manager_rx) = mpsc::channel(100); 
    
        let inner = Arc::new(NetworkInner {
            network: network.clone(),
            to_manager_tx,
        });
    
        let handle = Self {
            inner: inner.clone(),
        };
    
        // Improve network event handling
        let network_clone = inner.network.clone();
        tokio::spawn(async move {
            loop {
                match tokio::time::timeout(
                    Duration::from_secs(300),
                    network_clone.lock()
                ).await {
                    Ok(mut network_guard) => {
                        let event = network_guard.get_next_event().await;
                        network_guard.handle_swarm_event(event).await;
                    }
                    Err(e) => {
                        error!("Failed to acquire network lock: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(120)).await;
                    }
                }
            }
        });
    
        handle
    }
    
    /// Add new listener
    pub async fn add_listener(&self, listener_id: ListenerId, addr: Multiaddr) {
        self.inner.network.lock().await.swarm.lock().await.listen_on(addr);
    }

    /// Start listening on address
    pub async fn start_listening(&self, addr: Multiaddr) -> Result<(), Box<dyn Error + Send>> {
        let mut network = self.inner.network.lock().await;
        let _ = network.swarm.lock().await.listen_on(addr);
        Ok(())
    }

    pub async fn peer_info(&self) -> Result<PeerInfo, Box<dyn Error>> {
        Ok(self.inner.network.lock().await.peer_info().await)
    }

    /// Handle negotiation request
    pub async fn handle_negotiation_request(
        &self,
        from: PeerId,
        request_id: InboundRequestId,
        request: NegotiationRequest,
        channel: ResponseChannel<NegotiationResponse>,
    ) {
        self.inner.network.lock().await.handle_negotiation_request(from, request_id, request, channel);
    }

    /// Handle negotiation response
    pub async fn handle_negotiation_response(
        &self,
        request_id: OutboundRequestId,
        response: NegotiationResponse,
    ) {
        self.inner.network.lock().await.handle_negotiation_response(request_id, response);
    }

    /// Handle gossipsub message
    pub async fn handle_gossipsub_message(
        &self,
        message: GossipsubMessage,
    ) -> Result<(), Box<dyn Error>> {
        let mut network = self.inner.network.lock().await;

        network.handle_gossipsub_message(message).await
    }

    /// Send completion confirmation acknowledgement
    pub async fn send_completion_confirmation_ack(&self, to: &PeerId, work_id: String, uri: String) {
        let mut network = self.inner.network.lock().await.swarm.lock().await.behaviour_mut()
        .conode.negotiation_protocol.send_request(to, NegotiationRequest::CompletionAcknowledgement(to.to_string(), work_id, uri));
    }

    /// Join gossip protocol
    pub async fn join_gossip_protocol(&self) -> Result<bool, SubscriptionError> {
        let mut network = self.inner.network.lock().await;
        network.join_gossip_protocol().await
    }

    /// Broadcast work to the network
    pub async fn broadcast_work(&self, work: WorkBroadcast) {
        self.inner.clone().network.lock().await.broadcast_work(&work);
    }

    /// Initiate negotiation with a peer
    pub async fn initiate_negotiation(
        &self,
        recipient: PeerId,
        job_id: String,
        negotiation: &mut Negotiation,
    ) {
        self.inner.clone().network.lock().await.initiate_negotiation(&recipient, &job_id, negotiation).await;
    }

    /// Request completion acknowledgement
    pub async fn request_completion_ack(&self, recipient: PeerId, negotiation: Negotiation) {
        self.inner.network.lock().await.request_completion_ack(&recipient, negotiation);
    }

    /// Send completion confirmation request
    pub async fn send_completion_confirmation_request(
        &self,
        recipient: PeerId,
        negotiation: String,
    ) {
        self.inner.network.lock().await.send_completion_confirmation_request(&recipient, &negotiation);
    }

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: String) {
        self.inner.network.lock().await.swarm.lock().await.behaviour_mut().conode.gossipsub.subscribe(&work_discovery_topic());
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: String) {
        self.inner.network.lock().await.swarm.lock().await.behaviour_mut().conode.gossipsub.unsubscribe(&work_discovery_topic());
    }

    /// Shutdown the network
    pub async fn shutdown(&self) {
        self.inner.network.lock().await.shutdown();
    }

    /// Get the local PeerId
    pub async fn local_peer_id(&self) -> PeerId {
        *self.inner.network.lock().await.swarm.lock().await.local_peer_id()
    }

    pub async fn local_peer_info(&self) -> PeerInfo {
        self.inner.network.lock().await.peer_info().await
    }

    pub async fn add_peer_to_kademlia_dht(&self, peer_id: &PeerId, address: Multiaddr) -> Result<RoutingUpdate, Box<dyn Error>> {
        let mut network = self.inner.network.lock().await;

        let routing_update = network
            .swarm
            .lock().await.behaviour_mut()
            .conode
            .kademlia
            .add_address(peer_id, address);

        Ok(routing_update)
    }

    pub async fn bootstrap_kademlia(&self) {
        let network = self.inner.network.lock().await;
        // We don't handle the routing update here. The network will receive a swarm event
        // with the routing update if successful.
        let _ = network.swarm.lock().await.behaviour_mut().conode.kademlia.bootstrap();
    }
}
