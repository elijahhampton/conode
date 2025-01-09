use conode_types::work::{Work, WorkBroadcast};
use libp2p::gossipsub::Message as GossipsubMessage;
use std::error::Error;

/// Represents a discover in the network that can discover and handle
/// received broadcasted work.
pub trait Discoverer {
    /// Handles work broadcast according to a nodes configuration.
    ///
    /// # Arguments
    /// * `work` - The work broadcast received from the node.
    ///
    /// # Returns
    /// A future that resolves to the boolean indicating if the broadcast was handled correctly
    /// or not.
    fn handle_potential_work(
        &self,
        work: &WorkBroadcast,
        message: &GossipsubMessage,
    ) -> impl std::future::Future<Output = Result<bool, Box<dyn Error>>> + Send;

    /// Checks to see if a work item matches the nodes crtieria.
    ///
    /// # Arguments
    /// `work` - The work item received.
    ///
    /// # Returns
    /// Returns true if the work item matches the nodes work criteria and false
    /// otherwise.
    fn is_matching_node_critera(&self, work: &Work) -> bool;
}

/// Represents a seeker in the network that can broadcast work.
/// Handles the network communication aspects of work distribution.
pub trait Seeker {
    /// Broadcasts work information to the network under the work discovery topic.
    ///
    /// # Arguments
    /// * `data` - The work broadcast data containing work and peer information
    ///
    /// # Returns
    /// A future that resolves to a boolean indicating if the broadcast was successful
    fn broadcast_work<'a>(
        &'a mut self,
        data: &'a WorkBroadcast,
    ) -> impl std::future::Future<Output = bool> + Send + 'a;
}
