use super::CoNodeEvent;
use conode_types::negotiation::{NegotiationRequest, NegotiationResponse};
use libp2p::request_response::Event as RequestResponseEvent;

/// Convert RequestResponse events to CoNode event
impl From<RequestResponseEvent<NegotiationRequest, NegotiationResponse>> for CoNodeEvent {
    fn from(event: RequestResponseEvent<NegotiationRequest, NegotiationResponse>) -> Self {
        CoNodeEvent::NegotiationRequestResponse(event)
    }
}
