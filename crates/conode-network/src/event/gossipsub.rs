use libp2p::gossipsub::Event as GossipsubEvent;

use super::CoNodeEvent;

/// Convert Gossipsub events into custom CoNodeEvents
impl From<GossipsubEvent> for CoNodeEvent {
    fn from(event: GossipsubEvent) -> Self {
        CoNodeEvent::Gossipsub(event)
    }
}
