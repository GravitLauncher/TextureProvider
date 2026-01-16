pub mod backend;
pub mod local;
pub mod s3;

pub use backend::StorageBackend;
pub use local::LocalStorage;
pub use s3::S3Storage;

use crate::config::Config;
use std::sync::Arc;

/// Factory function to create the appropriate storage backend
pub fn create_storage(config: Config) -> Arc<dyn StorageBackend> {
    match config.storage_type {
        crate::config::StorageType::Local => Arc::new(LocalStorage::new(config)),
        crate::config::StorageType::S3 => Arc::new(S3Storage::new(config)),
    }
}
