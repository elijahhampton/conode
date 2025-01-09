use libp2p::gossipsub::Message;
use std::error::Error;

/// Methods related to libp2p::Gossipsub in relation to how the network implements this
/// protocol.
pub trait GossipsubNodeProvider {
    /// Handles Gossipsub messages for each topic the node is subscribed to.
    fn handle_gossipsub_message(
        & self,
        message: Message,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + Send;
    /// Subscribes the node to a topic.
    fn join_gossip_protocol(
        &mut self,
    ) -> impl std::future::Future<Output = Result<bool, libp2p::gossipsub::SubscriptionError>> + Send;
}
