use conode_types::{
    negotiation::{Negotiation, NegotiationResponse},
    work::{Work, WorkBroadcast},
};
use libp2p::PeerId;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum NodeCommand {
    // Commands related to job management
    BroadcastJob(WorkBroadcast),
    Jobs,

    // Commands related to negotiation
    InitiateNegotiation(PeerId, String, Negotiation), // Changed to owned types

    // System commands
    Stop(oneshot::Sender<()>),

    // Commands related to GossipSub/Discovery
    Topics,
    CreateTopic(String),
    SubscribeToTopic(String),
    UnsubscribeFromTopic(String),
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    // Events related to jobs
    JobReceived(Work),
    JobBroadcastConfirmed(String), // Job ID

    // Events related to negotiation
    NegotiationResponse(NegotiationResponse),
    NegotiationInitiated(PeerId, String), // Peer ID and Job ID

    // Events related to network status
    RelayConnected(String),
    RelayDisconnected(String),
    TopicSubscribed(String),
    TopicUnsubscribed(String),

    // Error events
    Error(String),

    // Shutdown
    ShutdownComplete,

    SyncStarted {
        from_block: u64,
        to_block: u64,
    },
    SyncProgress {
        current_block: u64,
        target_block: u64,
        percentage: u8,
    },
    SyncCompleted,
    SyncFailed(String),
}
