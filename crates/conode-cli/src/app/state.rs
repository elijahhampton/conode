// src/app/state.rs
use crate::types::enums::ExpiryOption;

#[derive(Debug, Clone)]
pub struct BroadcastFormState {
    pub expiry_date: Option<ExpiryOption>,
    pub reward: String,
    pub requirements: String,
    pub description: String,
}
