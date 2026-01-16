use super::backend::StorageBackend;
use crate::config::Config;
use anyhow::Result;
use async_trait::async_trait;

#[cfg(feature = "s3")]
use aws_config::BehaviorVersion;
#[cfg(feature = "s3")]
use aws_sdk_s3::{config::Credentials, primitives::ByteStream};

pub struct S3Storage {
    bucket: String,
    region: String,
    endpoint: Option<String>,
    credentials: Option<S3Credentials>,
}

struct S3Credentials {
    access_key: String,
    secret_key: String,
}

impl S3Storage {
    pub fn new(config: Config) -> Self {
        S3Storage {
            bucket: config
                .s3_bucket
                .expect("S3 bucket must be configured for S3 storage"),
            region: config.s3_region.unwrap_or_else(|| "us-east-1".to_string()),
            endpoint: config.s3_endpoint,
            credentials: match (config.s3_access_key, config.s3_secret_key) {
                (Some(access), Some(secret)) => Some(S3Credentials {
                    access_key: access,
                    secret_key: secret,
                }),
                _ => None,
            },
        }
    }

    /// Get or create AWS S3 client
    #[cfg(feature = "s3")]
    async fn get_client(&self) -> Result<aws_sdk_s3::Client> {
        use aws_sdk_s3::Client;

        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());

        // Add credentials if provided
        if let Some(creds) = &self.credentials {
            config_loader = config_loader.credentials_provider(Credentials::new(
                &creds.access_key,
                &creds.secret_key,
                None,
                None,
                "static",
            ));
        }

        let config = config_loader.load().await;
        Ok(Client::new(&config))
    }

    #[cfg(not(feature = "s3"))]
    async fn get_client(&self) -> Result<()> {
        Err(anyhow::anyhow!("S3 feature not enabled"))
    }

    /// Get file path in S3 bucket
    fn get_file_path(&self, hash: &str, extension: &str) -> String {
        format!("{}.{}", hash, extension)
    }

    /// Generate S3 URL
    fn generate_s3_url(&self, path: &str) -> String {
        if let Some(endpoint) = &self.endpoint {
            // Custom S3 endpoint
            format!("{}/{}", endpoint.trim_end_matches('/'), path)
        } else {
            // Standard AWS S3 URL
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                self.bucket, self.region, path
            )
        }
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn store_file(&self, bytes: Vec<u8>, hash: &str, extension: &str) -> Result<String> {
        #[cfg(feature = "s3")]
        {
            use aws_sdk_s3::primitives::ByteStream;

            let client = self.get_client().await?;
            let path = self.get_file_path(hash, extension);

            client
                .put_object()
                .bucket(&self.bucket)
                .key(&path)
                .body(ByteStream::from(bytes))
                .content_type("image/png")
                .send()
                .await?;

            Ok(self.generate_s3_url(&path))
        }

        #[cfg(not(feature = "s3"))]
        {
            Err(anyhow::anyhow!("S3 feature not enabled"))
        }
    }

    async fn get_file(&self, hash: &str, extension: &str) -> Result<Vec<u8>> {
        #[cfg(feature = "s3")]
        {
            let client = self.get_client().await?;
            let path = self.get_file_path(hash, extension);

            let response = client
                .get_object()
                .bucket(&self.bucket)
                .key(&path)
                .send()
                .await?;

            let bytes = response.body.collect().await?.into_bytes();
            Ok(bytes.to_vec())
        }

        #[cfg(not(feature = "s3"))]
        {
            Err(anyhow::anyhow!("S3 feature not enabled"))
        }
    }

    fn generate_url(&self, hash: &str, extension: &str) -> String {
        let path = self.get_file_path(hash, extension);
        self.generate_s3_url(&path)
    }
}
