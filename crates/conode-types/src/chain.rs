use serde::{Deserialize, Serialize};

/// A configuration containing parameters to interact with the blockchain
/// for this node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainConfig {
    pub conode_contract: String,
    pub payment_token_contract: String,
    pub chain_id: String,
    pub rpc: String,
}
