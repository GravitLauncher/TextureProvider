pub mod backend;
pub mod mojang;
pub mod storage_retriever;
pub mod default_skin;

pub use backend::TextureRetriever;
pub use mojang::MojangRetriever;
pub use storage_retriever::StorageRetriever;
pub use default_skin::DefaultSkinRetriever;

use crate::config::Config;
use std::sync::Arc;

/// Factory function to create the appropriate texture retriever based on configuration
pub fn create_retriever(
    config: Config,
    storage: Arc<dyn crate::storage::StorageBackend>,
    db: sqlx::PgPool,
) -> Arc<dyn TextureRetriever> {
    match config.retrieval_type {
        crate::config::RetrievalType::Storage => {
            Arc::new(StorageRetriever::new(storage, db))
        }
        crate::config::RetrievalType::Mojang => {
            Arc::new(MojangRetriever::new(config))
        }
        crate::config::RetrievalType::DefaultSkin => {
            Arc::new(DefaultSkinRetriever::new())
        }
    }
}
