use crate::config::{Config, StorageType};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Storage {
    config: Config,
}

impl Storage {
    pub fn new(config: Config) -> Self {
        Storage { config }
    }

    /// Calculate SHA256 hash of file bytes
    pub fn calculate_hash(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    /// Store a file and return its URL
    pub async fn store_file(&self, bytes: Vec<u8>, hash: &str) -> Result<String> {
        match self.config.storage_type {
            StorageType::Local => self.store_local(bytes, hash).await,
            StorageType::S3 => {
                #[cfg(feature = "s3")]
                {
                    self.store_s3(bytes, hash).await
                }
                #[cfg(not(feature = "s3"))]
                {
                    Err(anyhow::anyhow!("S3 feature not enabled"))
                }
            }
        }
    }

    /// Get file bytes by hash
    pub async fn get_file(&self, hash: &str) -> Result<Vec<u8>> {
        match self.config.storage_type {
            StorageType::Local => self.get_local(hash).await,
            StorageType::S3 => {
                #[cfg(feature = "s3")]
                {
                    self.get_s3(hash).await
                }
                #[cfg(not(feature = "s3"))]
                {
                    Err(anyhow::anyhow!("S3 feature not enabled"))
                }
            }
        }
    }

    /// Generate URL for a file by hash
    pub fn generate_url(&self, hash: &str) -> String {
        format!(
            "{}/{}.png",
            self.config.base_url.trim_end_matches('/'),
            hash
        )
    }

    /// Store file locally
    async fn store_local(&self, bytes: Vec<u8>, hash: &str) -> Result<String> {
        let storage_path = self
            .config
            .local_storage_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Local storage path not configured"))?;

        let path = PathBuf::from(storage_path);
        tokio::fs::create_dir_all(&path).await?;

        let file_path = path.join(format!("{}.png", hash));
        tokio::fs::write(&file_path, bytes).await?;

        Ok(self.generate_url(&hash))
    }

    /// Get file from local storage
    async fn get_local(&self, hash: &str) -> Result<Vec<u8>> {
        let storage_path = self
            .config
            .local_storage_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Local storage path not configured"))?;

        let file_path = PathBuf::from(storage_path).join(format!("{}.png", hash));
        tokio::fs::read(&file_path).await.map_err(|e| {
            anyhow::anyhow!("Failed to read file {}: {}", file_path.display(), e)
        })
    }

    /// Store file in S3
    #[cfg(feature = "s3")]
    async fn store_s3(&self, bytes: Vec<u8>, hash: &str) -> Result<String> {
        use aws_config::BehaviorVersion;
        use aws_sdk_s3::{primitives::ByteStream, Client};

        let bucket = self
            .config
            .s3_bucket
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("S3 bucket not configured"))?;

        // Load AWS config
        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());
        
        if let (Some(access_key), Some(secret_key)) = 
            (&self.config.s3_access_key, &self.config.s3_secret_key) {
            config_loader = config_loader.credentials_provider(
                aws_sdk_s3::config::Credentials::new(
                    access_key,
                    secret_key,
                    None,
                    None,
                    "static",
                )
            );
        }

        let config = config_loader.load().await;
        let client = Client::new(&config);

        let path = format!("{}.png", hash);

        client
            .put_object()
            .bucket(bucket)
            .key(&path)
            .body(ByteStream::from(bytes))
            .content_type("image/png")
            .send()
            .await?;

        Ok(format!(
            "https://{}.s3.{}.amazonaws.com/{}",
            bucket,
            self.config.s3_region.as_deref().unwrap_or("us-east-1"),
            path
        ))
    }

    /// Get file from S3
    #[cfg(feature = "s3")]
    async fn get_s3(&self, hash: &str) -> Result<Vec<u8>> {
        use aws_config::BehaviorVersion;
        use aws_sdk_s3::Client;

        let bucket = self
            .config
            .s3_bucket
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("S3 bucket not configured"))?;

        // Load AWS config
        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());
        
        if let (Some(access_key), Some(secret_key)) = 
            (&self.config.s3_access_key, &self.config.s3_secret_key) {
            config_loader = config_loader.credentials_provider(
                aws_sdk_s3::config::Credentials::new(
                    access_key,
                    secret_key,
                    None,
                    None,
                    "static",
                )
            );
        }

        let config = config_loader.load().await;
        let client = Client::new(&config);

        let path = format!("{}.png", hash);

        let response = client
            .get_object()
            .bucket(bucket)
            .key(&path)
            .send()
            .await?;

        let bytes = response.body.collect().await?.into_bytes();
        Ok(bytes.to_vec())
    }
}