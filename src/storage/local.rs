use super::backend::StorageBackend;
use crate::config::Config;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

pub struct LocalStorage {
    storage_path: PathBuf,
    base_url: String,
}

impl LocalStorage {
    pub fn new(config: Config) -> Self {
        let storage_path = config
            .local_storage_path
            .expect("Local storage path must be configured for Local storage");
        
        LocalStorage {
            storage_path: PathBuf::from(storage_path),
            base_url: config.base_url,
        }
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn store_file(&self, bytes: Vec<u8>, hash: &str, extension: &str) -> Result<String> {
        // Create directory if it doesn't exist
        tokio::fs::create_dir_all(&self.storage_path).await?;

        let file_name = format!("{}", hash);
        let file_path = self.storage_path.join(&file_name);
        
        tokio::fs::write(&file_path, bytes).await?;

        Ok(self.generate_url(hash, extension))
    }

    async fn get_file(&self, hash: &str, _extension: &str) -> Result<Vec<u8>> {
        let file_name = format!("{}", hash);
        let file_path = self.storage_path.join(&file_name);
        
        tokio::fs::read(&file_path).await.map_err(|e| {
            anyhow::anyhow!("Failed to read file {}: {}", file_path.display(), e)
        })
    }

    fn generate_url(&self, hash: &str, _extension: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            hash
        )
    }
}
