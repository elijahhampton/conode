use chrono::{self, DateTime, Utc};
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

/// Information regarding a peer's connectivity data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub connected_addr: Option<Multiaddr>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_seen: DateTime<Utc>,
    pub is_dedicated: bool,
}

/// Impl PartialEq for PeerInfo
impl PartialEq for PeerInfo {
    fn eq(&self, other: &PeerInfo) -> bool {
        self.peer_id == other.peer_id
    }
}
