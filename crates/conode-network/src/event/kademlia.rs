use libp2p_kad::Event;

use super::CoNodeEvent;

/// Convert Kademlia events into custom CoNodeEvents
impl From<Event> for CoNodeEvent {
    fn from(value: Event) -> Self {
        CoNodeEvent::Kademlia(value)
    }
}
