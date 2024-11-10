pub mod distributed;
pub mod gossipsub;
pub mod kademlia;
pub mod mdns;
pub mod request_response;

use conode_types::negotiation::{NegotiationRequest, NegotiationResponse};
use libp2p::{
    autonat, gossipsub::Event as GossipsubEvent, identify, kad::Event as KademliaEvent,
    mdns::Event as MdnsEvent, request_response::Event as RequestResponseEvent,
};

/// Event type covering core protocol behaviours
#[derive(Debug)]
pub enum CoNodeEvent {
    Gossipsub(GossipsubEvent),
    NegotiationRequestResponse(RequestResponseEvent<NegotiationRequest, NegotiationResponse>),
    Kademlia(KademliaEvent),
    Mdns(MdnsEvent),
}

/// Event type covering distributed behaviour events including the `CoNodeEvent`
pub enum DistributedBehaviourEvent {
    CoNode(CoNodeEvent),
    Identify(identify::Event),
    AutoNat(autonat::Event),
}
