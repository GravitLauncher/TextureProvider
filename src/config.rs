use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_public_key: String,
    pub base_url: String,
    pub storage_type: StorageType,
    pub retrieval_type: RetrievalType,
    pub retrieval_chain: Option<Vec<RetrievalType>>,
    pub local_storage_path: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub server_port: u16,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum StorageType {
    Local,
    S3,
}

impl std::str::FromStr for StorageType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(StorageType::Local),
            "s3" => Ok(StorageType::S3),
            _ => Err(anyhow::anyhow!("Invalid storage type: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum RetrievalType {
    Storage,
    Mojang,
    DefaultSkin,
}

impl std::str::FromStr for RetrievalType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "storage" => Ok(RetrievalType::Storage),
            "mojang" => Ok(RetrievalType::Mojang),
            "default_skin" => Ok(RetrievalType::DefaultSkin),
            _ => Err(anyhow::anyhow!("Invalid retrieval type: {}", s)),
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        // Parse retrieval_chain from comma-separated list if provided
        let retrieval_chain = env::var("RETRIEVAL_CHAIN").ok().map(|chain_str| {
            chain_str
                .split(',')
                .map(|s| s.trim().parse::<RetrievalType>())
                .collect::<Result<Vec<_>, _>>()
        }).transpose()?;

        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?,
            jwt_public_key: env::var("JWT_PUBLIC_KEY")
                .map_err(|_| anyhow::anyhow!("JWT_PUBLIC_KEY must be set"))?,
            base_url: env::var("BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            storage_type: env::var("STORAGE_TYPE")
                .unwrap_or_else(|_| "local".to_string())
                .parse()?,
            retrieval_type: env::var("RETRIEVAL_TYPE")
                .unwrap_or_else(|_| "storage".to_string())
                .parse()?,
            retrieval_chain,
            local_storage_path: env::var("LOCAL_STORAGE_PATH").ok(),
            s3_bucket: env::var("S3_BUCKET").ok(),
            s3_region: env::var("S3_REGION").ok(),
            s3_endpoint: env::var("S3_ENDPOINT").ok(),
            s3_access_key: env::var("S3_ACCESS_KEY").ok(),
            s3_secret_key: env::var("S3_SECRET_KEY").ok(),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid SERVER_PORT: {}", e))?,
        })
    }

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.storage_type == StorageType::Local {
            if self.local_storage_path.is_none() {
                return Err(anyhow::anyhow!(
                    "LOCAL_STORAGE_PATH must be set for local storage"
                ));
            }
        } else if self.storage_type == StorageType::S3 {
            if self.s3_bucket.is_none() {
                return Err(anyhow::anyhow!("S3_BUCKET must be set for S3 storage"));
            }
        }
        Ok(())
    }
}
