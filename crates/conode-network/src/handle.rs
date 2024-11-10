use crate::network::CoNodeNetwork;
use crate::protocol::discovery::traits::Seeker;
use crate::protocol::negotiation::traits::Negotiator;

use conode_types::peer::PeerInfo;

use conode_types::traits::libp2p::GossipsubNodeProvider;
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
use log::{error, warn};
use std::time::Duration;
use std::{error::Error, sync::Arc};
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot, Mutex,
};

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
    network: Arc<Mutex<CoNodeNetwork>>,
    /// Channel to send messages to the network manager
    to_manager_tx: UnboundedSender<NetworkHandleMessage>,
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
    pub fn new(network: Arc<Mutex<CoNodeNetwork>>) -> Self {
        let (to_manager_tx, mut to_manager_rx) = mpsc::unbounded_channel();

        let inner = Arc::new(NetworkInner {
            network: network.clone(),
            to_manager_tx,
        });

        let handle = Self {
            inner: inner.clone(),
        };

        let handle_clone = handle.clone();

        // Spawn a new thread to continuously handle swarm and network events.
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // First branch: handle events from the swarm
                    swarm_event = async {
                        // Here we acquire the lock only to get the next event
                        let next_event = {
                            let mut network = handle_clone.inner.network.lock().await;
                            network.swarm.select_next_some().await
                        };
                        // The network lock is released here while we process the event
                        next_event
                    } => {
                        if let Err(e) = network.lock().await.handle_swarm_event(swarm_event).await {
                            warn!("Error handling swarm event: {:?}", e);
                        }

                    }
                    // Second branch: handle incoming messages
                    Some(msg) = to_manager_rx.recv() => {
                        handle_clone.handle_message(msg).await;
                    }
                    // Frequently sleep this loop to allow other parts of the application
                    // to lock the swarm.
                    _ = tokio::time::sleep(Duration::from_millis(10)) => (),
                    else => break,
                }
            }
        });

        handle
    }

    /// Add new listener
    pub async fn add_listener(&self, listener_id: ListenerId, addr: Multiaddr) {
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::AddListener(listener_id, addr));
    }

    /// Start listening on address
    pub async fn start_listening(&self, addr: Multiaddr) -> Result<(), Box<dyn Error + Send>> {
        let mut network = self.inner.network.lock().await;
        let _ = network.swarm.listen_on(addr);
        Ok(())
    }

    pub async fn peer_info(&self) -> Result<PeerInfo, Box<dyn Error>> {
        Ok(self.inner.network.lock().await.peer_info())
    }

    /// Handle negotiation request
    pub async fn handle_negotiation_request(
        &self,
        from: PeerId,
        request_id: InboundRequestId,
        request: NegotiationRequest,
        channel: ResponseChannel<NegotiationResponse>,
    ) {
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::HandleNegotiationRequest {
                from,
                request_id,
                request,
                channel,
            });
    }

    /// Handle negotiation response
    pub async fn handle_negotiation_response(
        &self,
        request_id: OutboundRequestId,
        response: NegotiationResponse,
    ) {
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::HandleNegotiationResponse {
                request_id,
                response,
            });
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
    pub fn send_completion_confirmation_ack(&mut self, to: &PeerId, work_id: String, uri: String) {
        let _ =
            self.inner
                .to_manager_tx
                .send(NetworkHandleMessage::SendCompletionConfirmationAck {
                    to: *to,
                    work_id,
                    solution_uri: uri,
                });
    }

    /// Join gossip protocol
    pub async fn join_gossip_protocol(&self) -> Result<bool, SubscriptionError> {
        let mut network = self.inner.network.lock().await;
        network.join_gossip_protocol().await
    }

    /// Broadcast work to the network
    pub async fn broadcast_work(&self, work: WorkBroadcast) {
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::BroadcastWork(work));
    }

    /// Initiate negotiation with a peer
    pub async fn initiate_negotiation(
        &self,
        recipient: PeerId,
        job_id: String,
        negotiation: Negotiation,
    ) {
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::InitiateNegotiation {
                recipient,
                job_id,
                negotiation,
            });
    }

    /// Request completion acknowledgement
    pub async fn request_completion_ack(&self, recipient: PeerId, negotiation: Negotiation) {
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::RequestCompletionAck {
                recipient,
                negotiation,
            });
    }

    /// Send completion confirmation request
    pub async fn send_completion_confirmation_request(
        &self,
        recipient: PeerId,
        negotiation: String,
    ) {
        let _ = self.inner.to_manager_tx.send(
            NetworkHandleMessage::SendCompletionConfirmationRequest {
                recipient,
                negotiation,
            },
        );
    }

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: String) {
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::Subscribe(topic));
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: String) {
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::Unsubscribe(topic));
    }

    /// Shutdown the network
    pub async fn shutdown(&self) -> Result<(), oneshot::error::RecvError> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .inner
            .to_manager_tx
            .send(NetworkHandleMessage::Shutdown(tx));
        rx.await
    }

    /// Get the local PeerId
    pub async fn local_peer_id(&self) -> PeerId {
        *self.inner.network.lock().await.swarm.local_peer_id()
    }

    pub async fn add_peer_to_kademlia_dht(&self, peer_id: &PeerId, address: Multiaddr) {
        let mut network = self.inner.network.lock().await;
        // We don't handle the routing update here. The network will receive a swarm event
        // with the routing update if successful.
        let _ = network
            .swarm
            .behaviour_mut()
            .conode
            .kademlia
            .add_address(peer_id, address);
    }

    pub async fn bootstrap_kademlia(&self) {
        let mut network = self.inner.network.lock().await;
        // We don't handle the routing update here. The network will receive a swarm event
        // with the routing update if successful.
        let _ = network.swarm.behaviour_mut().conode.kademlia.bootstrap();
    }
}

impl NetworkHandle {
    /// Process a received network handle message
    pub async fn handle_message(&self, msg: NetworkHandleMessage) {
        match msg {
            NetworkHandleMessage::BroadcastWork(work) => {
                let mut network = self.inner.network.lock().await;
                let _ = network.broadcast_work(&work).await;
            }
            NetworkHandleMessage::InitiateNegotiation {
                recipient,
                job_id,
                mut negotiation,
            } => {
                let mut network = self.inner.network.lock().await;
                let _ = network
                    .initiate_negotiation(&recipient, &job_id, &mut negotiation)
                    .await;
            }
            NetworkHandleMessage::RequestCompletionAck {
                recipient,
                negotiation,
            } => {
                let mut network = self.inner.network.lock().await;
                network
                    .request_completion_ack(&recipient, negotiation)
                    .await;
            }
            NetworkHandleMessage::SendCompletionConfirmationRequest {
                recipient,
                negotiation,
            } => {
                let mut network = self.inner.network.lock().await;
                network
                    .send_completion_confirmation_request(&recipient, &negotiation)
                    .await;
            }
            NetworkHandleMessage::SendCompletionConfirmationAck {
                to,
                work_id,
                solution_uri,
            } => {
                let mut network = self.inner.network.lock().await;
                network
                    .send_completion_confirmation_ack(&to, work_id, solution_uri)
                    .await;
            }
            NetworkHandleMessage::HandleNegotiationRequest {
                from,
                request_id,
                request,
                channel,
            } => {
                let mut network = self.inner.network.lock().await;
                network
                    .handle_negotiation_request(from, request_id, request, channel)
                    .await;
            }
            NetworkHandleMessage::HandleNegotiationResponse {
                request_id,
                response,
            } => {
                let mut network = self.inner.network.lock().await;
                network
                    .handle_negotiation_response(request_id, response)
                    .await;
            }
            NetworkHandleMessage::HandleGossipsubMessage(message) => {
                let mut network = self.inner.network.lock().await;
                let _ = network.handle_gossipsub_message(message).await;
            }
            NetworkHandleMessage::Subscribe(topic) => {
                let mut network = self.inner.network.lock().await;
                let _ = network
                    .swarm
                    .behaviour_mut()
                    .conode
                    .gossipsub
                    .subscribe(&Sha256Topic::new(topic));
            }
            NetworkHandleMessage::Unsubscribe(topic) => {
                let mut network = self.inner.network.lock().await;
                let _ = network
                    .swarm
                    .behaviour_mut()
                    .conode
                    .gossipsub
                    .unsubscribe(&Sha256Topic::new(topic));
            }
            NetworkHandleMessage::AddListener(_listener_id, _addr) => {}
            NetworkHandleMessage::StartListening(addr) => {
                let mut network = self.inner.network.lock().await;
                let _ = network.swarm.listen_on(addr);
            }
            NetworkHandleMessage::JoinGossipProtocol => {
                let mut network = self.inner.network.lock().await;
                let _ = network.join_gossip_protocol();
            }
            NetworkHandleMessage::Shutdown(sender) => {
                if let Err(e) = self.inner.network.lock().await.shutdown().await {
                    error!("Network did not shutdown successfully {}", e.to_string());
                }
                let _ = sender.send(());
            }
        }
    }
}
