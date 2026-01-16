use anyhow::Result;
use async_trait::async_trait;

/// Trait defining the interface for storage backends
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a file and return its URL
    async fn store_file(&self, bytes: Vec<u8>, hash: &str, extension: &str) -> Result<String>;

    /// Get file bytes by hash
    async fn get_file(&self, hash: &str, extension: &str) -> Result<Vec<u8>>;

    /// Generate URL for a file by hash
    fn generate_url(&self, hash: &str, extension: &str) -> String;

    /// Calculate SHA256 hash of file bytes
    fn calculate_hash(&self, bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }
}
