use super::{CoNodeEvent, DistributedBehaviourEvent};
use libp2p::{autonat, identify};

/// Event conversion implementations for different libp2p protocol events into our
/// unified DistributedBehaviourEvent enum. This allows us to handle all network events
/// through a single event type.

/// Convert custom CoNode events into the distributed behaviour event
impl From<CoNodeEvent> for DistributedBehaviourEvent {
    fn from(event: CoNodeEvent) -> Self {
        DistributedBehaviourEvent::CoNode(event)
    }
}

/// Convert identify protocol events
/// These events provide information about peer identification and observed addresses
impl From<identify::Event> for DistributedBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        DistributedBehaviourEvent::Identify(event)
    }
}

/// Convert AutoNAT protocol events
/// These events relate to NAT traversal status and network reachability detection
impl From<autonat::Event> for DistributedBehaviourEvent {
    fn from(event: autonat::Event) -> Self {
        DistributedBehaviourEvent::AutoNat(event)
    }
}
