use starknet::core::types::{
    DeclareTransactionV3, DeployAccountTransactionV3, InvokeTransactionV3,
};

/// Starknet transaction primitives encapsulated.
pub enum StarknetTransaction {
    Invoke(InvokeTransactionV3),
    Deploy(DeployAccountTransactionV3),
    Declare(DeclareTransactionV3),
}
