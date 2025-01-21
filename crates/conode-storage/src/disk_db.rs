use crate::error::WorkManagerError;
use crate::traits::Storage;
use crate::traits::{StorageDefault, StorageKey};
use conode_config::configuration_exporter::ConfigurationExporter;
use rocksdb::{Options, DB};

// Concrete implementation
pub struct DiskDb {
    db: DB,
}

impl StorageDefault for DiskDb {
    fn default() -> Self {
        let configuration = ConfigurationExporter::new()
            .map_err(|e| WorkManagerError::Internal(format!("{}", e.to_string())))
            .unwrap();
        let data_dir = configuration.config.data_dir;

        let opts = Options::default();
        Self {
            db: DB::open_for_read_only(&opts, data_dir, false).unwrap(),
        }
    }
}

impl Storage for DiskDb {
    fn get_key(
        &self,
        key: &[u8],
        cf_name: Option<&str>,
    ) -> Result<Option<Vec<u8>>, WorkManagerError> {
        if let Some(cf_name) = cf_name {
            let cf_handle = self
                .db
                .cf_handle(cf_name)
                .expect("expect cf_handle to exist");
            let opt_bytes = self.db.get_cf(&cf_handle, key).unwrap_or_default();
            bincode::deserialize(&opt_bytes.unwrap_or_default())
                .map_err(|e| WorkManagerError::SerdeError(e.to_string()))
        } else {
            let opt_bytes = self.db.get(key).unwrap_or_default();
            bincode::deserialize(&opt_bytes.unwrap_or_default())
                .map_err(|e| WorkManagerError::SerdeError(e.to_string()))
        }
    }

    fn put_key(
        &self,
        key: &[u8],
        value: &[u8],
        cf_name: Option<&str>,
    ) -> Result<(), WorkManagerError> {
        if let Some(cf_name) = cf_name {
            let cf_handle = self
                .db
                .cf_handle(cf_name)
                .expect("expect cf_handle to exist");
            self.db
                .put_cf(&cf_handle, key, value)
                .map_err(|e| WorkManagerError::SerdeError(e.to_string()))
        } else {
            self.db
                .put(key, value)
                .map_err(|e| WorkManagerError::SerdeError(e.to_string()))
        }
    }

    fn delete_key(&self, key: &[u8], cf_name: Option<&str>) -> Result<(), WorkManagerError> {
        if let Some(cf_name) = cf_name {
            let cf_handle = self
                .db
                .cf_handle(cf_name)
                .expect("expect cf_handle to exist");
            self.db
                .delete_cf(&cf_handle, key)
                .map_err(|e| WorkManagerError::SerdeError(e.to_string()))
        } else {
            self.db
                .delete(key)
                .map_err(|e| WorkManagerError::SerdeError(e.to_string()))
        }
    }

    fn batch_write(
        &self,
        cf_name: Option<&str>,
        batch: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    ) -> Result<(), WorkManagerError> {
        let mut write_batch = rocksdb::WriteBatch::default();

        if let Some(cf_name) = cf_name {
            let cf_handle = self
                .db
                .cf_handle(cf_name)
                .expect("expect cf_handle to exist");

            for (key, value_opt) in batch {
                match value_opt {
                    Some(value) => write_batch.put_cf(&cf_handle, key, value),
                    None => write_batch.delete_cf(&cf_handle, key),
                }
            }
        } else {
            for (key, value_opt) in batch {
                match value_opt {
                    Some(value) => write_batch.put(&key, value),
                    None => write_batch.delete(&key),
                }
            }
        }

        self.db
            .write(write_batch)
            .map_err(|e| WorkManagerError::SerdeError(e.to_string()))
    }
}

// Implement AsRef<[u8]> for common types if needed
impl StorageKey for Vec<u8> {
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

impl StorageKey for String {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl StorageKey for &str {
    fn as_bytes(&self) -> &[u8] {
        str::as_bytes(self)
    }
}
