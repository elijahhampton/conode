use crate::error::WorkManagerError;

// src/storage/traits.rs
pub trait StorageKey {
    fn as_bytes(&self) -> &[u8];
}

pub trait Storage {
    fn get_key(&self, key: &[u8], cf_name: Option<&str>) -> Result<Option<Vec<u8>>, WorkManagerError>;
    fn put_key(&self, key: &[u8], value: &[u8], cf_name: Option<&str>) -> Result<(), WorkManagerError>;
    fn delete_key(&self, key: &[u8], cf_name: Option<&str>) -> Result<(), WorkManagerError>;
    fn batch_write(&self, cf_name: Option<&str>, batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<(), WorkManagerError>;
}

pub trait StorageDefault {
    fn default() -> Self;
}

pub trait StorageWithDefault: Storage + StorageDefault {}
impl<T: Storage + StorageDefault> StorageWithDefault for T {}