use starknet::core::types::Felt;

pub trait ChainItem {
    fn to_calldata(&self) -> Result<Vec<Felt>, Box<dyn std::error::Error>>;
}
