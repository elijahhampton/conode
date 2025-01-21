use crate::behaviour::DistributedBehaviour;
use crate::network::{Network, NetworkEvent};
use crate::protocol::discovery::traits::Seeker;
use crate::protocol::discovery::work_discovery_topic;
use crate::protocol::negotiation::traits::Negotiator;

use conode_storage::error::WorkManagerError;
use conode_types::peer::PeerInfo;

use conode_types::sync::SyncEvent;
use conode_types::traits::libp2p::GossipsubNodeProvider;
use conode_types::work::FlattenedWork;
use conode_types::{
    negotiation::{Negotiation, NegotiationRequest, NegotiationResponse},
    work::WorkBroadcast,
};
use futures::StreamExt;

use libp2p::Swarm;
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

pub trait NetworkStorage: Storage + Send + Sync + 'static {
    fn as_storage(&self) -> &dyn Storage;
}

impl Storage for Box<dyn NetworkStorage> {
    fn get_key(
        &self,
        key: &[u8],
        cf_name: Option<&str>,
    ) -> Result<Option<Vec<u8>>, WorkManagerError> {
        self.as_storage().get_key(key, cf_name)
    }

    fn put_key(
        &self,
        key: &[u8],
        value: &[u8],
        cf_name: Option<&str>,
    ) -> Result<(), WorkManagerError> {
        self.as_storage().put_key(key, value, cf_name)
    }

    fn delete_key(&self, key: &[u8], cf_name: Option<&str>) -> Result<(), WorkManagerError> {
        self.as_storage().delete_key(key, cf_name)
    }

    fn batch_write(
        &self,
        cf_name: Option<&str>,
        batch: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    ) -> Result<(), WorkManagerError> {
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
    network_tx: mpsc::Sender<NetworkEvent>,
}

/// NetworkInner contains the underlying network struct as well as a unbounder sender
/// that relays messages to the network for further handling.
struct NetworkInner {
    /// The network instance
    network: Arc<Mutex<Network>>,
    peer_info: PeerInfo,
}

impl NetworkHandle {
    /// Creates a new instance of NetworkHandle
    pub async fn new(network: Arc<Mutex<Network>>, network_tx: mpsc::Sender<NetworkEvent>) -> Self {
        let peer_info = network.lock().await.peer_info().await;

        let inner = Arc::new(NetworkInner {
            network: network.clone(),
            peer_info,
        });

        let handle = Self {
            inner: inner.clone(),
            network_tx,
        };

        handle
    }

    /// Start listening on address
    pub async fn start_listening(&self, addr: Multiaddr) {
        let _ = self.network_tx.send(NetworkEvent::Listen(addr)).await;
    }

    /// Get the peer info for this node
    pub async fn peer_info(&self) -> &PeerInfo {
        &self.inner.peer_info
    }

    /// Send completion confirmation acknowledgement
    pub async fn send_completion_confirmation_ack(
        &self,
        to: &PeerId,
        id: String,
        solution: String,
    ) {
        let _ = self
            .network_tx
            .send(NetworkEvent::SendCompletionConfirmationAck(
                to.to_owned(),
                id,
                solution,
            ))
            .await;
    }

    /// Join gossip protocol
    pub async fn join_gossip_protocol(&self) {
        let _ = self.network_tx.send(NetworkEvent::Join).await;
    }

    /// Broadcast work to the network
    pub async fn broadcast_work(&self, work: WorkBroadcast) {
        let _ = self.network_tx.send(NetworkEvent::Publish(work)).await;
    }

    /// Initiate negotiation with a peer
    pub async fn initiate_negotiation(
        &self,
        recipient: PeerId,
        task_id: String,
        negotiation: Negotiation,
    ) {
        let _ = self
            .network_tx
            .send(NetworkEvent::InitiateNegotiation(
                recipient,
                task_id,
                negotiation,
            ))
            .await;
    }

    /// Request completion acknowledgement
    pub async fn request_completion_ack(&self, recipient: PeerId, negotiation: Negotiation) {
        let _ = self
            .network_tx
            .send(NetworkEvent::RequestCompletionAcknowledgement(
                recipient,
                negotiation,
            ))
            .await;
    }

    /// Send completion confirmation request
    pub async fn send_completion_confirmation_request(
        &self,
        recipient: PeerId,
        negotiation: String,
    ) {
        let _ = self
            .network_tx
            .send(NetworkEvent::SendCompletionConfirmationRequest(
                recipient,
                negotiation,
            ))
            .await;
    }

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: String) {
        let _ = self.network_tx.send(NetworkEvent::Join).await;
    }

    /// Shutdown the network
    pub async fn shutdown(&self) {
        let _ = self.network_tx.send(NetworkEvent::Shutdown).await;
    }

    /// Get the local PeerId
    pub async fn local_peer_id(&self) -> &PeerId {
        &self.inner.peer_info.peer_id
    }

    pub async fn add_peer_to_kademlia_dht(&self, peer_id: &PeerId, address: Multiaddr) {
        let _ = self
            .network_tx
            .send(NetworkEvent::AddPeerToKademlia(peer_id.to_owned(), address))
            .await;
    }

    pub async fn bootstrap_kademlia(&self) {
        let _ = self.network_tx.send(NetworkEvent::BootstrapKademlia).await;
    }
}
