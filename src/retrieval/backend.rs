use crate::models::{TextureMetadata, TextureType};
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

/// Trait defining the interface for texture retrieval strategies
/// This separates the concern of how textures are fetched from where they are stored
#[async_trait]
pub trait TextureRetriever: Send + Sync {
    /// Retrieve texture metadata for a user
    /// Returns None if the texture is not available from this retrieval source
    async fn get_texture(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>>;

    /// Retrieve texture file bytes for a user
    /// Returns None if the texture is not available from this retrieval source
    /// This is more efficient than downloading via URL for storage-based retrievers
    async fn get_texture_bytes(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTextureBytes>>;

    /// Retrieve texture file bytes by hash
    /// This allows retrievers with embedded data (like EmbeddedDefaultSkinRetriever) to provide bytes
    /// Returns None if the texture is not available from this retrieval source
    async fn get_texture_bytes_by_hash(&self, hash: &str) -> Result<Option<RetrievedTextureBytes>> {
        // Default implementation returns None for backward compatibility
        Ok(None)
    }

    /// Check if this retriever can provide the given texture type
    fn supports_texture_type(&self, texture_type: TextureType) -> bool;
}

/// Represents a successfully retrieved texture
#[derive(Debug, Clone)]
pub struct RetrievedTexture {
    /// URL where the texture can be downloaded
    pub url: String,
    /// SHA256 hash of the texture data
    pub hash: String,
    /// Optional metadata (e.g., model type for skins)
    pub metadata: Option<TextureMetadata>,
}

/// Represents a successfully retrieved texture with file bytes
#[derive(Debug, Clone)]
pub struct RetrievedTextureBytes {
    /// SHA256 hash of the texture data
    pub hash: String,
    /// File bytes of the texture
    pub bytes: Vec<u8>,
    /// Optional metadata (e.g., model type for skins)
    pub metadata: Option<TextureMetadata>,
}
