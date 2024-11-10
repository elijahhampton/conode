use crate::interface::CoNodeStarknetProvider;
use starknet::core::types::Felt;
use std::sync::Arc;

/// Starknet transaction receipt
#[derive(Debug)]
pub struct TransactionReceipt {
    _transaction_hash: Felt,
    _status: TransactionStatus,
}

/// Starknet transaction status
#[derive(Debug)]
pub enum TransactionStatus {
    Pending,
    Accepted,
    Rejected(String),
}

#[derive(Clone)]
/// Transaction executor for Starknet transactions
pub struct StarknetTransactionExecutor {
    pub chain_provider: Arc<CoNodeStarknetProvider>,
}

impl StarknetTransactionExecutor {
    pub fn new(provider: Arc<CoNodeStarknetProvider>) -> Self {
        Self {
            chain_provider: provider,
        }
    }
}
