use std::fmt;

// Trait defining common behavior for all completion solutions
pub trait CompletionSolution: fmt::Debug + Send + Sync {
    fn is_valid(&self) -> bool;
    fn as_bytes(&self) -> Vec<u8>;
}
