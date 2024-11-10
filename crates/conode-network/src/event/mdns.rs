use libp2p::mdns::Event as MdnsEvent;

use super::CoNodeEvent;

/// Convert MdnsEvents to CoNodeEvents
impl From<MdnsEvent> for CoNodeEvent {
    fn from(value: MdnsEvent) -> Self {
        CoNodeEvent::Mdns(value)
    }
}
