use std::collections::HashMap;

use super::backend::{
    download_file_from_url, RetrievedTexture, RetrievedTextureBytes, TextureRetriever,
};
use crate::config::Config;
use crate::models::{TextureMetadata, TextureType};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Retrieves textures from the Mojang API
/// This allows fetching official Minecraft skins and capes
pub struct MojangRetriever {
    client: reqwest::Client,
    api_base_url: String,
    session_server_url: String,
    use_database_username_in_mojang_requests: bool,
    db: Option<PgPool>,
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
    textures: HashMap<String, TextureData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TextureData {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<TextureMetadata>,
}

pub fn extract_hash_from_url(url: &str) -> Option<&str> {
    // Remove query parameters
    let url = url.split('?').next()?;

    // Get last path segment
    let last_segment = url.rsplit('/').next()?;

    // Remove file extension if present
    let hash = last_segment.split('.').next()?;

    if hash.is_empty() {
        None
    } else {
        Some(hash)
    }
}

impl MojangRetriever {
    pub fn new(config: Config, db: Option<PgPool>) -> Self {
        MojangRetriever {
            client: reqwest::Client::new(),
            api_base_url: "https://api.mojang.com".to_string(),
            session_server_url: "https://sessionserver.mojang.com/session/minecraft/profile"
                .to_string(),
            use_database_username_in_mojang_requests: config
                .use_database_username_in_mojang_requests,
            db: db,
        }
    }

    /// Resolve a username to UUID using Mojang API
    /// This is useful for legacy systems that only have usernames
    pub async fn resolve_username_to_uuid(&self, username: &str) -> Result<Option<Uuid>> {
        let url = format!(
            "{}/users/profiles/minecraft/{}",
            self.api_base_url, username
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to resolve username from Mojang: {}", e))?;

        // 204 No Content means user doesn't exist
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(anyhow!("Mojang API returned error: {}", response.status()));
        }

        #[derive(Deserialize)]
        struct UuidResponse {
            id: String,
        }

        let uuid_response: UuidResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse UUID response: {}", e))?;

        let uuid = Uuid::parse_str(&uuid_response.id)
            .map_err(|e| anyhow!("Failed to parse UUID: {}", e))?;

        Ok(Some(uuid))
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
            return Err(anyhow!("Mojang API returned error: {}", response.status()));
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

    async fn get_textures_from_mojang(
        &self,
        fetch_uuid: Uuid,
    ) -> Result<HashMap<String, RetrievedTexture>> {
        // Fetch profile from Mojang
        let profile = self.fetch_profile(fetch_uuid).await?;

        // Find textures property
        let textures_property = profile
            .properties
            .iter()
            .find(|p| p.name == "textures")
            .ok_or_else(|| anyhow!("Profile does not have textures property"))?;

        // Decode the base64-encoded textures
        let payload = Self::decode_textures_payload(&textures_property.value)?;

        Ok(payload
            .textures
            .iter()
            .map(|(k, v)| {
                let e = RetrievedTexture {
                    url: v.url.to_owned(),
                    hash: extract_hash_from_url(&v.url).map_or("", |e| e).to_owned(),
                    metadata: v.metadata.to_owned(),
                };
                (k.to_owned(), e)
            })
            .collect())
    }

    async fn get_texture_from_mojang(
        &self,
        fetch_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>> {
        // Only support SKIN and CAPE from Mojang
        match texture_type {
            TextureType::SKIN | TextureType::CAPE => {}
            _ => return Ok(None),
        }

        let mut textures = self.get_textures_from_mojang(fetch_uuid).await?;

        let key = match texture_type {
            TextureType::SKIN => "SKIN",
            TextureType::CAPE => "CAPE",
            _ => return Ok(None),
        };

        Ok(textures.remove(key))
    }

        async fn get_texture_bytes_from_mojang(
        &self,
        texture: &RetrievedTexture,
    ) -> Result<RetrievedTextureBytes> {
                // Download the texture bytes
                let response = self
                    .client
                    .get(&texture.url)
                    .send()
                    .await
                    .map_err(|e| anyhow!("Failed to download texture: {}", e))?;

                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| anyhow!("Failed to read texture bytes: {}", e))?
                    .to_vec();
        Ok(RetrievedTextureBytes {
                    hash: texture.hash.to_owned(),
                    bytes,
                    metadata: texture.metadata.to_owned(),
                })
    }
}

#[async_trait]
impl TextureRetriever for MojangRetriever {
    async fn get_textures(&self, user_uuid: Uuid) -> Result<HashMap<String, RetrievedTexture>> {
        let mut fetch_uuid = user_uuid;

        if self.use_database_username_in_mojang_requests {
            if let Some(db) = &self.db {
                match sqlx::query!(
                    r#"
                    SELECT username
                    FROM username_mappings
                    WHERE user_uuid = $1
                    LIMIT 1
                    "#,
                    user_uuid
                ).fetch_optional(db).await {
                    Ok(Some(record)) => {
                        match self.resolve_username_to_uuid(&record.username).await {
                            Ok(Some(resolved_uuid)) => {
                                fetch_uuid = resolved_uuid;
                            }
                            Ok(None) => {
                                tracing::warn!(
                                    "Username '{}' not found in Mojang API, using original UUID",
                                    record.username
                                );
                                // Keep using the original UUID
                            }
                            Err(e) => {
                                tracing::error!("Failed to resolve username from Mojang: {}", e);
                                // Continue with original UUID
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::debug!("No username mapping found for UUID {}", user_uuid);
                    }
                    Err(e) => {
                        tracing::error!("Failed to lookup username from database: {}", e);
                        // Continue with original UUID
                    }
                }
            }
        }

        self.get_textures_from_mojang(fetch_uuid).await
    }

    async fn get_texture_bytes(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTextureBytes>> {
        // For Mojang retriever, we need to download the texture bytes
        // First get the texture metadata
        let texture = self.get_texture(user_uuid, texture_type).await?;

        match texture {
            Some(texture) => {
                Ok(Some(self.get_texture_bytes_from_mojang(&texture).await?))
            }
            None => Ok(None),
        }
    }

    async fn get_texture_bytes_by_hash(&self, hash: &str) -> Result<Option<RetrievedTextureBytes>> {
        // Mojang textures follow the pattern: https://textures.minecraft.net/texture/SHA256_HASH
        let url = format!("https://textures.minecraft.net/texture/{}", hash);

        match download_file_from_url(&url).await? {
            Some(bytes) => Ok(Some(RetrievedTextureBytes {
                hash: hash.to_string(),
                bytes,
                metadata: None,
            })),
            None => Ok(None),
        }
    }

    fn supports_texture_type(&self, texture_type: TextureType) -> bool {
        matches!(texture_type, TextureType::SKIN | TextureType::CAPE)
    }

    async fn get_texture_bytes_by_username(
        &self,
        username: &str,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTextureBytes>> {
        // Only support SKIN and CAPE from Mojang
        if !matches!(texture_type, TextureType::SKIN | TextureType::CAPE) {
            return Ok(None);
        }

        // Resolve username to UUID
        let uuid = match self.resolve_username_to_uuid(username).await? {
            Some(uuid) => uuid,
            None => return Ok(None),
        };

        let texture = self.get_texture_from_mojang(uuid, texture_type).await?;
        match texture {
            Some(e) => {
                Ok(Some(self.get_texture_bytes_from_mojang(&e).await?))
            },
            None => Ok(None),
        }
    }
}
