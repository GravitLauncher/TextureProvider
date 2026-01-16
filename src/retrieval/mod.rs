pub mod backend;
pub mod chain;
pub mod mojang;
pub mod storage_retriever;
pub mod default_skin;

pub use backend::TextureRetriever;
pub use chain::ChainRetriever;
pub use mojang::MojangRetriever;
pub use storage_retriever::StorageRetriever;
pub use default_skin::DefaultSkinRetriever;

use crate::config::{Config, RetrievalType};
use std::sync::Arc;

/// Factory function to create the appropriate texture retriever based on configuration
/// If retrieval_chain is configured, returns a ChainRetriever with all handlers in order
/// Otherwise, returns a single retriever based on retrieval_type
pub fn create_retriever(
    config: Config,
    storage: Arc<dyn crate::storage::StorageBackend>,
    db: sqlx::PgPool,
) -> Arc<dyn TextureRetriever> {
    // If retrieval_chain is configured, build a chain of retrievers
    if let Some(chain_types) = &config.retrieval_chain {
        if chain_types.is_empty() {
            tracing::warn!("RETRIEVAL_CHAIN is empty, falling back to single retriever");
            return create_single_retriever(&config, storage, db);
        }

        tracing::info!(
            "Creating retrieval chain with {} handlers: {:?}",
            chain_types.len(),
            chain_types
        );

        let handlers: Vec<Arc<dyn TextureRetriever>> = chain_types
            .iter()
            .map(|retrieval_type| create_retriever_by_type(retrieval_type, &config, storage.clone(), db.clone()))
            .collect();

        tracing::info!(
            "Created chain retriever with {} handlers in order",
            handlers.len()
        );

        return Arc::new(ChainRetriever::new(handlers));
    }

    // Fallback to single retriever based on retrieval_type
    create_single_retriever(&config, storage, db)
}

/// Create a single retriever based on the retrieval_type
fn create_single_retriever(
    config: &Config,
    storage: Arc<dyn crate::storage::StorageBackend>,
    db: sqlx::PgPool,
) -> Arc<dyn TextureRetriever> {
    tracing::info!("Creating single retriever of type: {:?}", config.retrieval_type);
    create_retriever_by_type(&config.retrieval_type, config, storage, db)
}

/// Create a retriever for a specific retrieval type
fn create_retriever_by_type(
    retrieval_type: &RetrievalType,
    config: &Config,
    storage: Arc<dyn crate::storage::StorageBackend>,
    db: sqlx::PgPool,
) -> Arc<dyn TextureRetriever> {
    match retrieval_type {
        RetrievalType::Storage => {
            tracing::debug!("Creating StorageRetriever");
            Arc::new(StorageRetriever::new(storage, db))
        }
        RetrievalType::Mojang => {
            tracing::debug!("Creating MojangRetriever");
            Arc::new(MojangRetriever::new(config.clone()))
        }
        RetrievalType::DefaultSkin => {
            tracing::debug!("Creating DefaultSkinRetriever");
            Arc::new(DefaultSkinRetriever::new())
        }
    }
}
