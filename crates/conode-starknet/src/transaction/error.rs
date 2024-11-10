/// Enum representing the possible errors returned from starknet execution
/// transactions, i.e. starknet::execute, starknet::execute_v3
pub enum TransactionError {
    InvalidSignature,
    InsufficientBalance,
    ContractError(String),
}
