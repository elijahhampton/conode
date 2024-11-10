use libp2p::gossipsub::Sha256Topic;

pub mod traits;

/// A libp2p topic representing a channel for work discovery
pub fn work_discovery_topic() -> Sha256Topic {
    let sha256_topic = Sha256Topic::new("CONODE://WORK_DISCOVERY");
    sha256_topic
}
