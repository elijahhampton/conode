use libp2p::{
    autonat,
    gossipsub::{Behaviour as Gossipsub, ConfigBuilder, MessageAuthenticity, ValidationMode},
    identify,
    kad::{store::MemoryStore, Behaviour as Kademlia},
    mdns::{tokio::Behaviour as Mdns, Config as MdnsConfig},
    request_response::{Behaviour as RequestResponse, Config, ProtocolSupport},
    swarm::NetworkBehaviour,
    PeerId,
};

use super::protocol::negotiation::codecs::negotation::{
    NegotiationCodec, NEGOTATION_REQUEST_RESPONSE_PROTOCOL,
};

use super::event::CoNodeEvent;
use super::event::DistributedBehaviourEvent;

/// CoNodeNetworkBehaviour supporting GossipSub, Mdns, Kaemlia, RequestResponse
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "CoNodeEvent")]
pub struct CoNodeBehaviour {
    pub gossipsub: Gossipsub,
    pub negotiation_protocol: RequestResponse<NegotiationCodec>,
    pub mdns: Mdns,
    pub kademlia: Kademlia<MemoryStore>,
}

/// CoNodeDistributedNetworkBehaviour supporting CoNode network behaviour as well as
/// identify, autonat, relay and dcutr.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "DistributedBehaviourEvent")]
pub struct DistributedBehaviour {
    pub conode: CoNodeBehaviour,
    pub identify: identify::Behaviour,
    pub autonat: autonat::Behaviour,
}

impl DistributedBehaviour {
    pub fn new(local_peer_id: PeerId, local_key: libp2p::identity::Keypair) -> Self {
        let conode = CoNodeBehaviour::new(local_peer_id, local_key.clone());

        let identify = identify::Behaviour::new(identify::Config::new(
            "/ipfs/id/1.0.0".into(),
            local_key.public(),
        ));

        let autonat = autonat::Behaviour::new(local_peer_id, Default::default());

        Self {
            conode,
            identify,
            autonat,
        }
    }
}

impl CoNodeBehaviour {
    pub fn new(local_peer_id: PeerId, local_key: libp2p::identity::Keypair) -> Self {
        let gossipsub_config = ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .build()
            .expect("Failed to create gossipsub config.");

        Self {
            gossipsub: Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config)
                .expect("Failed to create Gossipsub instance."),
            negotiation_protocol: RequestResponse::new(
                [(NEGOTATION_REQUEST_RESPONSE_PROTOCOL, ProtocolSupport::Full)],
                Config::default(),
            ),
            mdns: Mdns::new(MdnsConfig::default(), local_peer_id).unwrap(),
            kademlia: Kademlia::new(local_peer_id, MemoryStore::new(local_peer_id)),
        }
    }
}
