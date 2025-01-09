use crate::{
    crypto::{ECDSASignature, FeltWrapper}, peer::PeerInfo, traits::common::Unqiue, work::FlattenedWork
};

use libp2p::PeerId;
use serde::{
    de::{self, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};
use uuid::Uuid;

/// Enum representing the status of a proposal in the negotiation process.
/// Currently includes SENT and ACKNOWLEDGED states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Initial state when a proposal is first created and sent by a worker
    /// Corresponds to NewProposal request being sent
    Proposed,

    /// The proposal has been received and acknowledged by the employer
    /// Set after ProposalAcknowledgement response is received
    Acknowledged,

    /// The employer has signed the proposal and requested completion acknowledgement
    /// Set when RequestCompletionAcknowledgement is sent
    /// Contains the employer's signature
    EmployerSigned(ECDSASignature),

    /// Both parties have signed the proposal (fully signed)
    /// Set after AcknowledgementReceived with worker's signature
    /// Contains both signatures for verification
    /// This is the last step in the proposal process and all states are transitioned
    /// on chain
    FullySigned {
        employer_signature: ECDSASignature,

        worker_signature: ECDSASignature,
    },

    /// An error occurred during the negotiation process
    /// Used for handling edge cases and invalid state transitions
    Error(String),
}

impl Serialize for ECDSASignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as a byte sequence
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.r)?;
        seq.serialize_element(&self.s)?;
        seq.serialize_element(&self.v)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for ECDSASignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize as a sequence
        struct ECDSASignatureVisitor;

        impl<'de> Visitor<'de> for ECDSASignatureVisitor {
            type Value = ECDSASignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of 3 Felt values")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let r = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let s = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let v = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;

                Ok(ECDSASignature { r, s, v })
            }
        }

        deserializer.deserialize_seq(ECDSASignatureVisitor)
    }
}

impl ProposalStatus {
    /// Check if the proposal can transition to the given new status
    pub fn can_transition_to(&self, new_status: &ProposalStatus) -> bool {
        use ProposalStatus::*;

        match (self, new_status) {
            // Initial proposal flow
            (Proposed, Acknowledged) => true,
            (Acknowledged, EmployerSigned(_)) => true,
            (EmployerSigned(_), FullySigned { .. }) => true,

            // Error can happen from any state
            (_, Error(_)) => true,

            // All other transitions are invalid
            _ => false,
        }
    }

    /// Returns whether both parties have signed
    pub fn is_fully_signed(&self) -> bool {
        matches!(self, ProposalStatus::FullySigned { .. })
    }
}

/// Request structure for the NEGOTIATION protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Negotiation {
    pub id: String,
    pub job_id: String,
    pub proposed_payout: Option<u64>,
    pub employer_signature: Option<ECDSASignature>,
    pub worker_signature: Option<ECDSASignature>,
    pub worker_public_key: Option<FeltWrapper>,
    pub employer: PeerId,
    pub status: Option<ProposalStatus>,
    pub employer_peer_info: Option<PeerInfo>,
    pub worker_peer_info: Option<PeerInfo>,
    pub worker_address: Option<FeltWrapper>,
}

impl Unqiue for Negotiation {
    fn id(&self) -> &String {
        &self.id
    }
}

impl Negotiation {
    /// Constructor for creating a new, empty Negotiation instance
    pub fn new(
        employer_peer_info: Option<PeerInfo>,
        worker_peer_info: Option<PeerInfo>,
        employer: PeerId,
        job_id: String,
        proposed_payout: Option<u64>,
    ) -> Self {
        Negotiation {
            id: Uuid::new_v4().to_string(),
            job_id,
            proposed_payout,
            employer_signature: None,
            worker_public_key: None,
            worker_signature: None,
            status: None,
            employer,
            employer_peer_info,
            worker_peer_info,
            worker_address: None,
        }
    }
}

/// Enum representing various types of requests that can occur during the negotiation process.
/// Includes steps like proposing, acknowledging completion, and making decisions.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NegotiationRequest {
    // Represents a new proposal received from an interested node
    // [Potential Worker sends]
    NewProposal(PeerId, Negotiation), // (from, negotiation)

    // In this step the employing node creates a transaction on chain solidifying
    // the work is active with a record
    // [Employer]
    RequestCompletionAcknowledgement(PeerId, String, FeltWrapper, ECDSASignature), // (From, Negotiation_ID, PublicKey as Felt, Employer Negotiation Signature)

    // [Worker]
    CompletionConfirmation(FlattenedWork),

    // [Worker]
    // In place of a string for the solution we will eventually implement the Solution trait
    // which will allow us to use dynamic dispatch here for a variety of solution types.
    CompletionAcknowledgement(String, String, String), // (WorkId, Solution URI, Salt)
}

/// Enum representing responses to negotiation requests.
/// Currently only includes acknowledgement of completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NegotiationResponse {
    // An acknowledgement from the employing node that a proposal was received.
    ProposalAcknowledgement(PeerId, String), // (From, NegotiationId)
    // [Worker Sent Response] Response to RequestCompletionAcknowledgement
    AcknowledgementReceived(PeerId, String, ECDSASignature, FeltWrapper), // (From, Negotiation_ID, Worker Negotiation Signature, Worker Address)
    ActiveWorkAcknowledgement(FlattenedWork),
    SolutionAck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    Accept(String),
    Reject,
}
