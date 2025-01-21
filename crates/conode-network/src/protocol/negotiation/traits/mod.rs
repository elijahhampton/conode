use conode_types::negotiation::{Negotiation, NegotiationRequest, NegotiationResponse};
use libp2p::{
    request_response::{InboundRequestId, OutboundRequestId, ResponseChannel},
    PeerId,
};

use crate::network::NetworkError;

/// Defines the interface for handling work negotiations between peers in the network.
/// This includes initiating negotiations, handling requests/responses, and managing
/// completion acknowledgments.
pub trait Negotiator {
    /// Initiates a new negotiation with a peer. This function is invoked by a `worker`
    /// and sent to an `employing` node after receiving a `WorkBroadcast` that
    /// is eligible for storage.
    ///
    /// # Arguments
    /// * `peer_id` - The ID of the peer to negotiate with
    /// * `job_id` - Unique identifier for the job being negotiated
    /// * `negotiation` - The negotiation details and terms
    fn initiate_negotiation(
        &mut self,
        peer_id: &PeerId,
        job_id: &String,
        negotiation: &mut Negotiation,
    ) -> impl std::future::Future<Output = Result<(), NetworkError>> + Send;

    /// Requests acknowledgment of work completion from a peer. This function is invoked by an
    /// `employing` node seeking to accept a negotiation. The `employing` node signs the received
    /// negotiation and sends the negotiation alongside its public key for verification.
    ///
    /// # Arguments
    /// * `recipient` - The peer to request acknowledgment from
    /// * `negotiation` - The completed negotiation details
    fn request_completion_ack(
        &mut self,
        recipient: &PeerId,
        negotiation: Negotiation,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Handles incoming negotiation requests from other peers.
    ///
    /// # Arguments
    /// * `from` - PeerId of the requesting peer
    /// * `request_id` - Unique identifier for this inbound request
    /// * `request` - The negotiation request details
    /// * `channel` - Channel for sending the response back
    fn handle_negotiation_request(
        &mut self,
        from: PeerId,
        request_id: InboundRequestId,
        request: NegotiationRequest,
        channel: ResponseChannel<NegotiationResponse>,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Handles incoming negotiation responses from other peers.
    ///
    /// # Arguments
    /// * `request_id` - Identifier of the original outbound request
    /// * `response` - The negotiation response received
    fn handle_negotiation_response(
        &self,
        request_id: OutboundRequestId,
        response: NegotiationResponse,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// This function is invoked by an `employing` node. Once the negotiation has been
    /// signed by both parties the `employing` node creates a transaction on chain
    /// officially beginning the active work period or deadline specified by the
    /// negotiation.
    ///
    /// # Arguments
    /// * `recipient` - The peer to send the confirmation request to
    /// * `negotiation` - Identifier of the negotiation to confirm
    fn send_completion_confirmation_request(
        &mut self,
        recipient: &PeerId,
        negotiation: &String,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// This function is invoked by a `worker` node. The `worker` node at this point
    /// will generate a random salt and hash the solution using a pederson_hash. The hash
    /// will be submitted to chain for storage and eventual verification from the
    /// `employing` node.
    ///
    /// # Arguments
    /// * `to` - The peer to send the acknowledgment to
    /// * `work_id` - Identifier of the completed work
    /// * `solution_uri` - URI where the solution can be found
    fn send_completion_confirmation_ack(
        &mut self,
        to: &PeerId,
        work_id: String,
        solution_uri: String,
    ) -> impl std::future::Future<Output = ()> + Send;
}
