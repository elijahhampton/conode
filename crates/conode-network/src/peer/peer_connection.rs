use libp2p::{
    core::{transport::{ListenerId, PortUse}, ConnectedPoint, Endpoint}, identify::Info, identity::PublicKey, mdns::Event as MdnsEvent, swarm::{ConnectionError, ConnectionId}, Multiaddr, PeerId, StreamProtocol
};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::{sync::{Mutex, RwLock}, time::Instant};
use libp2p_kad::KBucketDistance;
use libp2p::kad::Event as KadEvent;

#[derive(Debug, Clone)]
pub enum PeerState {
    Connected,
    Disconnected(Option<String>),
    Unroutable,
    PendingKadDiscovery,
    RoutingUpdated,
    MdnsDiscovered,
    MdnsExpired,
}

#[derive(Clone, Debug)]
pub enum ListenerStatus {
    Active,
    Expired,
    Closed(ListenerCloseReason),
    Error(String),
}

#[derive(Clone, Debug)]
pub enum ListenerCloseReason {
    Done,
    Error(String),
    Aborted,
    UserInitiated,
}

#[derive(Error, Debug)]
pub enum NetworkManagerError {
    #[error("Listener {0} not found")]
    ListenerNotFound(ListenerId),
    #[error("Address {1} not found for listener {0}")]
    AddressNotFound(ListenerId, Multiaddr),
    #[error("Failed to acquire lock: {0}")]
    LockError(String),
    #[error("Invalid state transition from {0:?} to {1:?}")]
    InvalidStateTransition(ListenerStatus, ListenerStatus),
}

#[derive(Debug, Clone)]
pub struct KadPeerInfo {
    pub addresses: Vec<Multiaddr>,
    pub last_seen: Instant,
    pub is_new_peer: bool,
    pub bucket_range: Option<(KBucketDistance, KBucketDistance)>,
}

pub struct PeerConnection {
    pub connection_id: ConnectionId,
    pub endpoint: ConnectedPoint,
    pub is_open: bool,
    pub public_key: Option<PublicKey>,
    pub listen_addrs: Option<Vec<Multiaddr>>,
    pub protocols: Option<Vec<StreamProtocol>>,
    pub observed_addr: Option<Multiaddr>,
    pub error: Option<ConnectionError>,
    pub state: PeerState,
    pub kad_info: Option<KadPeerInfo>,
}

impl PeerConnection {
    pub fn new(connection_id: ConnectionId, endpoint: ConnectedPoint) -> Self {
        Self {
            connection_id,
            endpoint,
            is_open: true,
            public_key: None,
            listen_addrs: None,
            protocols: None,
            observed_addr: None,
            error: None,
            state: PeerState::Connected,
            kad_info: None,
        }
    }

    pub fn update_kad_info(&mut self, info: KadPeerInfo) {
        self.kad_info = Some(info);
    }
}

pub struct PeerManager {
    connections: Arc<RwLock<HashMap<PeerId, PeerConnection>>>,
    identifications: Arc<RwLock<HashMap<PeerId, Info>>>,
    listener_statuses: Arc<Mutex<HashMap<ListenerId, Vec<(Multiaddr, ListenerStatus)>>>>,
}

impl Clone for PeerManager {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
            identifications: Arc::clone(&self.identifications),
            listener_statuses: Arc::clone(&self.listener_statuses),
        }
    }
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            identifications: Arc::new(RwLock::new(HashMap::new())),
            listener_statuses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    
    pub async fn new_peer_connection(&self, peer_id: PeerId, connection: PeerConnection) {
        self.connections.write().await.insert(peer_id, connection);
        self.connections
            .write()
            .await
            .entry(peer_id)
            .and_modify(|connection_info| {
                connection_info.is_open = true;
                connection_info.error = None;
            });
    }

    pub async fn close_peer_connection(&self, peer_id: &PeerId, cause: Option<ConnectionError>) {
        if let Some(conn) = self.connections.write().await.get_mut(peer_id) {
            conn.is_open = false;
            conn.state = match &cause {
                Some(err) => PeerState::Disconnected(Some(err.to_string())),
                None => PeerState::Disconnected(Some("Unknown reason".to_string())),
            };
            conn.error = cause;
        }
    }

    pub async fn new_identification(&self, peer_id: PeerId, info: Info) {
        self.identifications.write().await.insert(peer_id, info);
    }

    pub async fn add_peer_identity(&self, peer_id: &PeerId, info: Info) {
        if self.connections.read().await.contains_key(peer_id) {
            self.connections
                .write()
                .await
                .entry(peer_id.clone())
                .and_modify(|connection_info| {
                    connection_info.public_key = Some(info.public_key);
                    connection_info.listen_addrs = Some(info.listen_addrs);
                    connection_info.protocols = Some(info.protocols);
                    connection_info.observed_addr = Some(info.observed_addr);
                });
        }
    }

    // Event Handling Methods
    pub async fn handle_kad_event(&self, event: KadEvent) {
        match event {
            KadEvent::RoutingUpdated {
                peer,
                is_new_peer,
                addresses,
                bucket_range,
                old_peer,
            } => {
                let kad_info = KadPeerInfo {
                    addresses: addresses.into_vec(),
                    last_seen: Instant::now(),
                    is_new_peer,
                    bucket_range: Some(bucket_range),
                };

                let mut connections = self.connections.write().await;
                if let Some(connection) = connections.get_mut(&peer) {
                    connection.update_kad_info(kad_info);
                    connection.state = PeerState::RoutingUpdated;
                } else {
                    let mut new_conn = PeerConnection::new(
                        ConnectionId::new_unchecked(0),
                        ConnectedPoint::Dialer {
                            address: kad_info.addresses[0].clone(),
                            role_override: Endpoint::Dialer,
                            port_use: PortUse::Reuse,
                        },
                    );
                    new_conn.update_kad_info(kad_info);
                    new_conn.state = PeerState::PendingKadDiscovery;
                    connections.insert(peer, new_conn);
                }

                if let Some(old_peer) = old_peer {
                    if let Some(conn) = connections.get_mut(&old_peer) {
                        conn.state = PeerState::Disconnected(Some("Replaced by new peer".to_string()));
                    }
                }
            }
            KadEvent::UnroutablePeer { peer } => {
                if let Some(conn) = self.connections.write().await.get_mut(&peer) {
                    conn.state = PeerState::Unroutable;
                }
            }
            KadEvent::RoutablePeer { peer, address } => {
                let mut connections = self.connections.write().await;
                if let Some(conn) = connections.get_mut(&peer) {
                    if let Some(kad_info) = &mut conn.kad_info {
                        if !kad_info.addresses.contains(&address) {
                            kad_info.addresses.push(address);
                        }
                        kad_info.last_seen = Instant::now();
                    } else {
                        conn.kad_info = Some(KadPeerInfo {
                            addresses: vec![address],
                            last_seen: Instant::now(),
                            is_new_peer: true,
                            bucket_range: None,
                        });
                    }
                    conn.state = PeerState::Connected;
                }
            }
            _ => {}
        }
    }

    pub async fn handle_mdns_event(&self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(nodes) => {
                let mut connections = self.connections.write().await;
                for (peer_id, addr) in nodes {
                    if let Some(conn) = connections.get_mut(&peer_id) {
                        if let Some(kad_info) = &mut conn.kad_info {
                            if !kad_info.addresses.contains(&addr) {
                                kad_info.addresses.push(addr.clone());
                            }
                            kad_info.last_seen = Instant::now();
                        } else {
                            conn.kad_info = Some(KadPeerInfo {
                                addresses: vec![addr.clone()],
                                last_seen: Instant::now(),
                                is_new_peer: true,
                                bucket_range: None,
                            });
                        }
                        conn.state = PeerState::MdnsDiscovered;
                    } else {
                        let mut new_conn = PeerConnection::new(
                            ConnectionId::new_unchecked(0),
                            ConnectedPoint::Dialer {
                                address: addr.clone(),
                                role_override: Endpoint::Dialer,
                                port_use: PortUse::Reuse,
                            },
                        );
                        new_conn.kad_info = Some(KadPeerInfo {
                            addresses: vec![addr.clone()],
                            last_seen: Instant::now(),
                            is_new_peer: true,
                            bucket_range: None,
                        });
                        new_conn.state = PeerState::MdnsDiscovered;
                        connections.insert(peer_id, new_conn);
                    }
                }
            }
            MdnsEvent::Expired(nodes) => {
                let mut connections = self.connections.write().await;
                for (peer_id, addr) in nodes {
                    if let Some(conn) = connections.get_mut(&peer_id) {
                        if let Some(kad_info) = &mut conn.kad_info {
                            kad_info.addresses.retain(|a| a != &addr);
                            if kad_info.addresses.is_empty() {
                                conn.state = PeerState::MdnsExpired;
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn new_listen_addr(&self, listener_id: ListenerId, addr: Multiaddr) {
        let mut statuses = self.listener_statuses.lock().await;
        let addresses = statuses.entry(listener_id).or_insert_with(Vec::new);
        addresses.push((addr, ListenerStatus::Active));
    }

    pub async fn close_listener(
        &self,
        listener_id: ListenerId,
        reason: ListenerCloseReason,
    ) -> Result<(), NetworkManagerError> {
        let mut statuses = self.listener_statuses.lock().await;
        let addresses = statuses
            .get_mut(&listener_id)
            .ok_or(NetworkManagerError::ListenerNotFound(listener_id))?;

        for (_, status) in addresses.iter_mut() {
            *status = ListenerStatus::Closed(reason.clone());
        }
        Ok(())
    }

    pub async fn set_listener_error(
        &self,
        listener_id: ListenerId,
        error: impl std::fmt::Display,
    ) -> Result<(), NetworkManagerError> {
        let mut statuses = self.listener_statuses.lock().await;
        let addresses = statuses
            .get_mut(&listener_id)
            .ok_or(NetworkManagerError::ListenerNotFound(listener_id))?;

        for (_, status) in addresses.iter_mut() {
            *status = ListenerStatus::Error(error.to_string());
        }
        Ok(())
    }

    // Query Methods
    pub async fn get_routable_peers(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter_map(|(peer_id, conn)| {
                conn.kad_info
                    .as_ref()
                    .map(|info| (*peer_id, info.addresses.clone()))
            })
            .collect()
    }

    pub async fn get_peer_state(&self, peer_id: &PeerId) -> Option<PeerState> {
        self.connections
            .read()
            .await
            .get(peer_id)
            .map(|conn| match &conn.state {
                state => state.clone(),
            })
    }

    pub async fn get_kad_info(&self, peer_id: &PeerId) -> Option<KadPeerInfo> {
        self.connections
            .read()
            .await
            .get(peer_id)
            .and_then(|conn| conn.kad_info.clone())
    }

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

    pub async fn remove_listener(&self, listener_id: ListenerId) {
        self.listener_statuses.lock().await.remove(&listener_id);
    }

    pub async fn clear_state(&self) {
        let mut statuses = self.listener_statuses.lock().await;
        for (_, addresses) in statuses.iter_mut() {
            for (_, status) in addresses.iter_mut() {
                if matches!(status, ListenerStatus::Active) {
                  //  *status = ListenerStatus::Closed(ListenerCloseReason::UserInitiated);
                }
            }
        }
        statuses.clear();
        
        let mut connections = self.connections.write().await;
        for (_, conn) in connections.iter_mut() {
            conn.state = PeerState::Disconnected(Some("Network shutdown".to_string()));
            conn.is_open = false;
        }
        connections.clear();
        
        self.identifications.write().await.clear();
    }
}