use super::backend::{RetrievedTexture, TextureRetriever};
use crate::config::Config;
use crate::models::{TextureMetadata, TextureType};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Retrieves textures from the Mojang API
/// This allows fetching official Minecraft skins and capes
pub struct MojangRetriever {
    client: reqwest::Client,
    api_base_url: String,
    session_server_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProfileResponse {
    id: String,
    name: String,
    properties: Vec<ProfileProperty>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProfileProperty {
    name: String,
    value: String,
    signature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TexturesPayload {
    textures: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct TextureData {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<TextureMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TextureMeta {
    model: String,
}

impl MojangRetriever {
    pub fn new(config: Config) -> Self {
        MojangRetriever {
            client: reqwest::Client::new(),
            api_base_url: "https://api.mojang.com".to_string(),
            session_server_url: "https://sessionserver.mojang.com/session/minecraft/profile".to_string(),
        }
    }

    /// Fetch the full profile from Mojang session server
    async fn fetch_profile(&self, uuid: Uuid) -> Result<ProfileResponse> {
        let url = format!("{}/{}", self.session_server_url, uuid);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch profile from Mojang: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Mojang API returned error: {}",
                response.status()
            ));
        }

        response
            .json::<ProfileResponse>()
            .await
            .map_err(|e| anyhow!("Failed to parse profile response: {}", e))
    }

    /// Decode Base64 texture payload
    fn decode_textures_payload(encoded: &str) -> Result<TexturesPayload> {
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| anyhow!("Failed to decode base64: {}", e))?;

        let payload: TexturesPayload = serde_json::from_slice(&decoded)
            .map_err(|e| anyhow!("Failed to parse textures payload: {}", e))?;

        Ok(payload)
    }
}

#[async_trait]
impl TextureRetriever for MojangRetriever {
    async fn get_texture(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>> {
        // Only support SKIN and CAPE from Mojang
        match texture_type {
            TextureType::SKIN | TextureType::CAPE => {}
            _ => return Ok(None),
        }

        // Fetch profile from Mojang
        let profile = self.fetch_profile(user_uuid).await?;

        // Find textures property
        let textures_property = profile
            .properties
            .iter()
            .find(|p| p.name == "textures")
            .ok_or_else(|| anyhow!("Profile does not have textures property"))?;

        // Decode the base64-encoded textures
        let payload = Self::decode_textures_payload(&textures_property.value)?;

        // Extract the specific texture
        let texture_key = match texture_type {
            TextureType::SKIN => "SKIN",
            TextureType::CAPE => "CAPE",
            _ => return Ok(None),
        };

        let texture_obj = payload
            .textures
            .get(texture_key.to_lowercase())
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow!("Texture {} not found in profile", texture_key))?;

        let texture_data: TextureData = serde_json::from_value(serde_json::to_value(texture_obj)?)?;

        // Extract metadata if present
        let metadata = texture_data.metadata.map(|m| TextureMetadata {
            model: Some(m.model),
        });

        // Download the texture to calculate hash
        let response = self
            .client
            .get(&texture_data.url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to download texture: {}", e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow!("Failed to read texture bytes: {}", e))?;

        // Calculate hash
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = hex::encode(hasher.finalize());

        Ok(Some(RetrievedTexture {
            url: texture_data.url,
            hash,
            metadata,
        }))
    }

    fn supports_texture_type(&self, texture_type: TextureType) -> bool {
        matches!(texture_type, TextureType::SKIN | TextureType::CAPE)
    }
}
