use crate::{
    crypto::{ECDSASignature, FeltWrapper},
    negotiation::Negotiation,
    peer::PeerInfo,
    traits::chain::ChainItem,
};
use chrono::{DateTime, Utc};

use libp2p::PeerId;

use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;
use uuid::Uuid;

/// Struct containing detailed metadata about a work item.
/// Includes description, requirements, chain reference, expiry date, reward, and status.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkMetadata {
    pub description: String,
    pub requirements: Vec<String>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub reward: Option<u64>,
    pub status: String,
}

/// Main struct representing a work item in the system.
/// Contains all necessary information about a job, including its metadata and associated addresses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Work {
    pub id: String,
    pub proposal_id: Option<String>,
    pub proposal_signatures: Option<(Felt, Felt)>, // (employer_signature, worker_signature)
    pub details: WorkMetadata,
    pub worker_address: Option<String>,
    pub employer_address: Option<String>,
}

/// A simplified version of the Work struct for more efficient storage or transmission.
/// This version is submitted on chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlattenedWork {
    pub id: String,
    pub proposal_signatures: Vec<ECDSASignature>,
    pub expiry_date: i64,
    pub reward: u64,
    pub worker_address: FeltWrapper,
}

impl FlattenedWork {
    pub fn task_id_felt_to_uuid(task_id: Felt) -> uuid::Uuid {
        let work_id_str = task_id.to_string();
        let work_id_u128 = work_id_str.parse::<u128>().unwrap();
        Uuid::from_u128(work_id_u128)
    }
}

impl From<Negotiation> for FlattenedWork {
    fn from(proposal: Negotiation) -> Self {
        let mut signatures = Vec::new();
        signatures.push(proposal.employer_signature.unwrap());
        signatures.push(proposal.worker_signature.unwrap());

        Self {
            id: Uuid::new_v4().to_string(),
            proposal_signatures: signatures,
            expiry_date: DateTime::from_timestamp_nanos(0).timestamp(),
            reward: proposal.proposed_payout.unwrap() as u64,
            worker_address: proposal.worker_address.unwrap().into(),
        }
    }
}

impl ChainItem for FlattenedWork {
    fn to_calldata(&self) -> Result<Vec<Felt>, Box<dyn std::error::Error>> {
        let mut calldata = Vec::new();

        // Convert the work id to a Felt for compatibilty with Starknet (String -> Uuid -> u128 -> Felt)
        let uuid = Uuid::parse_str(&self.id).unwrap();
        let uuid_int = uuid.as_u128();
        let id_felt = Felt::from(uuid_int);
        calldata.push(id_felt);

        // We instantiate the calldata for the employer adress to Felt::ZERO. The on chain contract will re assign
        // the emploer address once the transaction begins executing. The chain will check to make sure the initial
        // value is set to Felt::ZERO.
        // Emploer address and signature
        let employer = Felt::ZERO;
        calldata.push(employer);

        let employer_signature = self.proposal_signatures.get(0).unwrap();
        calldata.push(employer_signature.to_felt().unwrap());

        // Worker address and signature
        let worker = self.worker_address;
        calldata.push(worker.into());
        calldata.push(self.proposal_signatures.get(1).unwrap().to_felt().unwrap());

        // 4. reward
        calldata.push(Felt::from(self.reward));

        // 5. status
        calldata.push(Felt::from(0_u8));

        Ok(calldata)
    }
}

impl From<ECDSASignature> for Felt {
    fn from(sig: ECDSASignature) -> Self {
        // Implementation depends on your specific requirements
        // This might involve converting the signature components to a field element
        sig.r.0 + sig.s.0 + sig.v.0
    }
}
impl PartialEq for FlattenedWork {
    fn eq(&self, other: &FlattenedWork) -> bool {
        // First check if lengths match to avoid unnecessary iteration
        let is_proposal_signatures_eq_len =
            self.proposal_signatures.len() == other.proposal_signatures.len();
        if !is_proposal_signatures_eq_len {
            return false;
        }

        // Check if all signature Felts match
        let is_proposal_signatures_eq = self
            .proposal_signatures
            .iter()
            .zip(other.proposal_signatures.iter())
            .all(|(a, b)| a == b);

        // Only return true if IDs match and all signatures match
        self.id == other.id
            && is_proposal_signatures_eq
            && self.expiry_date == other.expiry_date
            && self.reward == other.reward
            && self.worker_address == other.worker_address
    }
}

impl Work {
    pub fn new(
        description: String,
        requirements: Vec<String>,
        expiry_date: Option<DateTime<Utc>>,
        reward: Option<u64>,
        proposal_id: Option<String>,
        employer_address: Option<String>,
        worker_address: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::max().to_string(),
            proposal_id,
            details: WorkMetadata {
                description,
                requirements,
                expiry_date,
                reward,
                status: "".to_string(),
            },
            employer_address,
            worker_address,
            proposal_signatures: None,
        }
    }
}

impl Into<Vec<u8>> for Work {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

/// Struct for broadcasting work items, including the work itself and information about the broadcasting peer.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkBroadcast {
    pub work: Work,
    pub peer_info: PeerInfo,
}

impl WorkBroadcast {
    pub fn new(work: Work, peer_info: PeerInfo) -> Self {
        Self { work, peer_info }
    }
}

impl Into<Vec<u8>> for WorkBroadcast {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum JobRole {
    Employer,
    Worker,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActiveWork {
    pub work: FlattenedWork,
    pub role: JobRole,
    pub employer_peer_id: PeerId,
    pub worker_peer_id: PeerId,
}

impl ActiveWork {
    pub fn new(
        work: FlattenedWork,
        role: JobRole,
        employer_peer_id: PeerId,
        worker_peer_id: PeerId,
    ) -> Self {
        Self {
            work,
            role,
            employer_peer_id,
            worker_peer_id,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PendingItem {
    pub work: WorkBroadcast,
    pub is_marked_for_save: bool,
}
