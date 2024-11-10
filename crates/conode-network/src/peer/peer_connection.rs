use libp2p::core::transport::PortUse;
use libp2p::kad::{Event as KadEvent, KBucketDistance};
use libp2p::{
    core::ConnectedPoint,
    identify::Info,
    identity::PublicKey,
    mdns::Event as MdnsEvent,
    swarm::{ConnectionError, ConnectionId},
    Multiaddr, PeerId, StreamProtocol,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{sync::RwLock, time::Instant};

#[derive(Debug)]
pub enum PeerState {
    Connected,
    Disconnected(Option<String>),
    Unroutable,
    PendingKadDiscovery,
    RoutingUpdated,
    MdnsDiscovered,
    MdnsExpired,
}

impl Clone for PeerState {
    fn clone(&self) -> Self {
        match self {
            Self::Connected => Self::Connected,
            Self::Disconnected(err) => Self::Disconnected(err.clone()),
            Self::Unroutable => Self::Unroutable,
            Self::PendingKadDiscovery => Self::PendingKadDiscovery,
            Self::RoutingUpdated => Self::RoutingUpdated,
            Self::MdnsDiscovered => Self::MdnsDiscovered,
            Self::MdnsExpired => Self::MdnsExpired,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KadPeerInfo {
    pub addresses: Vec<Multiaddr>,
    pub last_seen: Instant,
    pub is_new_peer: bool,
    pub bucket_range: Option<(KBucketDistance, KBucketDistance)>,
}

/// Represents a CoNode peer connection. This structure stores the necessary details
/// to check the state of a connection and potentially reconnect with a peer if that
/// connection is closed.
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
    // Create a new PeerConnection
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

/// A PeerManager that manages peer connections including connection details and
/// inbound/outbound queries.
pub struct PeerManager {
    connections: Arc<RwLock<HashMap<PeerId, PeerConnection>>>,
    identifications: Arc<RwLock<HashMap<PeerId, Info>>>,
}

impl Clone for PeerManager {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
            identifications: Arc::clone(&self.identifications),
        }
    }
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            identifications: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Stores data for a new peer connection
    pub async fn new_peer_connection(&self, peer_id: PeerId, connection: PeerConnection) {
        self.connections.write().await.insert(peer_id, connection);
        self.connections
            .write()
            .await
            .entry(peer_id.clone())
            .and_modify(|connection_info| {
                connection_info.is_open = true;
                connection_info.error = None;
            });
    }

    pub async fn close_peer_connection(&self, peer_id: &PeerId, cause: Option<ConnectionError>) {
        if let Some(conn) = self.connections.write().await.get_mut(peer_id) {
            conn.is_open = false;
            // Convert the error to string first if it exists
            conn.state = match &cause {
                Some(err) => PeerState::Disconnected(Some(err.to_string())),
                None => PeerState::Disconnected(Some("Unknown reason".to_string())),
            };
            conn.error = cause; // Move the original error, don't clone it
        }
    }

    pub async fn new_identification(&self, peer_id: PeerId, info: Info) {
        self.identifications.write().await.insert(peer_id, info);
    }

    /// Stores peer identity data into the hashmap for a peer_id
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
                    // Create a placeholder connection for Kademlia-discovered peers
                    let mut new_conn = PeerConnection::new(
                        ConnectionId::new_unchecked(0), // placeholder
                        ConnectedPoint::Dialer {
                            address: kad_info.addresses[0].clone(),
                            role_override: libp2p::core::Endpoint::Dialer,
                            port_use: PortUse::Reuse,
                        },
                    );
                    new_conn.update_kad_info(kad_info);
                    new_conn.state = PeerState::PendingKadDiscovery;
                    connections.insert(peer, new_conn);
                }

                if let Some(old_peer) = old_peer {
                    if let Some(conn) = connections.get_mut(&old_peer) {
                        conn.state =
                            PeerState::Disconnected(Some("Replaced by new peer".to_string()));
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

            KadEvent::PendingRoutablePeer { peer, address } => {
                let mut connections = self.connections.write().await;
                if !connections.contains_key(&peer) {
                    let mut new_conn = PeerConnection::new(
                        ConnectionId::new_unchecked(0),
                        ConnectedPoint::Dialer {
                            address: address.clone(),
                            role_override: libp2p::core::Endpoint::Dialer,
                            port_use: PortUse::Reuse,
                        },
                    );
                    new_conn.kad_info = Some(KadPeerInfo {
                        addresses: vec![address],
                        last_seen: Instant::now(),
                        is_new_peer: true,
                        bucket_range: None,
                    });
                    new_conn.state = PeerState::PendingKadDiscovery;
                    connections.insert(peer, new_conn);
                }
            }

            _ => {}
        }
    }

    /// Helper methods for querying peer information
    // Retrieve routable peers information, i,e, peer_id and a multi addresses
    pub async fn get_routable_peers(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter_map(|(peer_id, conn)| {
                if let Some(kad_info) = &conn.kad_info {
                    Some((*peer_id, kad_info.addresses.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    // Get the current peer state
    pub async fn get_peer_state(&self, peer_id: &PeerId) -> Option<PeerState> {
        self.connections
            .read()
            .await
            .get(peer_id)
            .map(|conn| conn.state.clone())
    }

    // Retrieve Kademlia info for a peer
    pub async fn get_kad_info(&self, peer_id: &PeerId) -> Option<KadPeerInfo> {
        self.connections
            .read()
            .await
            .get(peer_id)
            .and_then(|conn| conn.kad_info.clone())
    }

    pub async fn handle_mdns_event(&self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(nodes) => {
                let mut connections = self.connections.write().await;

                for (peer_id, addr) in nodes {
                    if let Some(conn) = connections.get_mut(&peer_id) {
                        // Update existing peer
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
                        // Create new peer connection
                        let mut new_conn = PeerConnection::new(
                            ConnectionId::new_unchecked(0),
                            ConnectedPoint::Dialer {
                                address: addr.clone(),
                                role_override: libp2p::core::Endpoint::Dialer,
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
                            // Remove the expired address
                            kad_info.addresses.retain(|a| a != &addr);

                            if kad_info.addresses.is_empty() {
                                // If no addresses left, mark peer as expired
                                conn.state = PeerState::MdnsExpired;
                            }
                        }
                    }
                }
            }
        }
    }

    // Helper method to get mDNS discovered peers
    pub async fn get_mdns_peers(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter_map(|(peer_id, conn)| {
                if matches!(conn.state, PeerState::MdnsDiscovered) {
                    conn.kad_info
                        .as_ref()
                        .map(|info| (*peer_id, info.addresses.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}
