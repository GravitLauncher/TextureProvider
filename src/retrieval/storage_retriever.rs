use super::backend::{RetrievedTexture, RetrievedTextureBytes, TextureRetriever};
use crate::models::{TextureMetadata, TextureType};
use crate::storage::StorageBackend;
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Retrieves textures from the storage backend (original behavior)
/// This wraps the existing storage and database logic
pub struct StorageRetriever {
    db: PgPool,
    storage: Arc<dyn StorageBackend>,
}

impl StorageRetriever {
    pub fn new(storage: Arc<dyn StorageBackend>, db: PgPool) -> Self {
        StorageRetriever {
            db,
            storage,
        }
    }
}

#[async_trait]
impl TextureRetriever for StorageRetriever {
    async fn get_texture(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>> {
        let texture = sqlx::query!(
            r#"
            SELECT file_hash, file_url, metadata
            FROM textures
            WHERE user_uuid = $1 AND texture_type = $2
            "#,
            user_uuid,
            texture_type.to_string()
        )
        .fetch_optional(&self.db)
        .await?;

        match texture {
            Some(texture) => {
                let metadata: Option<TextureMetadata> = texture
                    .metadata
                    .and_then(|v| serde_json::from_value(v).ok());

                Ok(Some(RetrievedTexture {
                    url: texture.file_url,
                    hash: texture.file_hash,
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_texture_bytes(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTextureBytes>> {
        let texture = sqlx::query!(
            r#"
            SELECT file_hash, metadata
            FROM textures
            WHERE user_uuid = $1 AND texture_type = $2
            "#,
            user_uuid,
            texture_type.to_string()
        )
        .fetch_optional(&self.db)
        .await?;

        match texture {
            Some(texture) => {
                let metadata: Option<TextureMetadata> = texture
                    .metadata
                    .and_then(|v| serde_json::from_value(v).ok());

                // Get file bytes from storage
                let bytes = self
                    .storage
                    .get_file(&texture.file_hash, texture_type.file_extension())
                    .await?;

                Ok(Some(RetrievedTextureBytes {
                    hash: texture.file_hash,
                    bytes,
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    fn supports_texture_type(&self, texture_type: TextureType) -> bool {
        // Storage retriever supports all texture types
        matches!(texture_type, TextureType::SKIN | TextureType::CAPE)
    }
}
