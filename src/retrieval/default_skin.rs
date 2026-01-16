use super::backend::{RetrievedTexture, TextureRetriever};
use crate::models::TextureType;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

/// Retrieves default/special textures
/// Provides the default Minecraft Steve skin for users without custom skins
pub struct DefaultSkinRetriever {
    // Pre-computed hash and URL for default Steve skin
    default_steve_url: String,
    default_steve_hash: String,
}

impl DefaultSkinRetriever {
    pub fn new() -> Self {
        // The official default Steve skin from Minecraft
        let default_steve_url = "http://textures.minecraft.net/texture/1a4af718455d58aab3011401517e43cb6f84b5f9cbd717f8df0334e0b88b8ecf".to_string();
        
        // Pre-computed hash of the default Steve skin
        let default_steve_hash = "1a4af718455d58aab3011401517e43cb6f84b5f9cbd717f8df0334e0b88b8ecf".to_string();

        DefaultSkinRetriever {
            default_steve_url,
            default_steve_hash,
        }
    }

    /// Create with custom default skin URL and hash
    pub fn with_custom_default(skin_url: String, skin_hash: String) -> Self {
        DefaultSkinRetriever {
            default_steve_url: skin_url,
            default_steve_hash: skin_hash,
        }
    }
}

impl Default for DefaultSkinRetriever {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TextureRetriever for DefaultSkinRetriever {
    async fn get_texture(
        &self,
        _user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>> {
        match texture_type {
            TextureType::SKIN => {
                // Return the default Steve skin for any user requesting a skin
                Ok(Some(RetrievedTexture {
                    url: self.default_steve_url.clone(),
                    hash: self.default_steve_hash.clone(),
                    metadata: None, // Default skin has no special metadata
                }))
            }
            TextureType::CAPE => {
                // Default cape doesn't exist, return None
                // Capes are optional in Minecraft
                Ok(None)
            }
        }
    }

    fn supports_texture_type(&self, texture_type: TextureType) -> bool {
        // Only supports SKIN type, not CAPE
        matches!(texture_type, TextureType::SKIN)
    }
}

/// Alternative implementation that returns embedded default skin bytes
/// This could be used if you want to serve the default skin directly from your server
pub struct EmbeddedDefaultSkinRetriever {
    default_skin_data: Vec<u8>,
    default_skin_hash: String,
    base_url: String,
}

impl EmbeddedDefaultSkinRetriever {
    /// Create with embedded default skin data
    /// You would embed the default skin bytes in the binary
    pub fn new(default_skin_data: Vec<u8>, base_url: String) -> Self {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(&default_skin_data);
        let hash = hex::encode(hasher.finalize());

        EmbeddedDefaultSkinRetriever {
            default_skin_data,
            default_skin_hash: hash,
            base_url,
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[async_trait]
impl TextureRetriever for EmbeddedDefaultSkinRetriever {
    async fn get_texture(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>> {
        match texture_type {
            TextureType::SKIN => {
                let url = format!(
                    "{}/default/skin/{}",
                    self.base_url,
                    user_uuid
                );
                
                Ok(Some(RetrievedTexture {
                    url,
                    hash: self.default_skin_hash.clone(),
                    metadata: None,
                }))
            }
            TextureType::CAPE => {
                Ok(None)
            }
        }
    }

    fn supports_texture_type(&self, texture_type: TextureType) -> bool {
        matches!(texture_type, TextureType::SKIN)
    }
}

impl EmbeddedDefaultSkinRetriever {
    // You could add a method to get the actual bytes
    pub fn get_default_skin_bytes(&self) -> &[u8] {
        &self.default_skin_data
    }
}
