use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Enum of supported texture types
/// To add a new texture type:
/// 1. Add a variant here
/// 2. Update the TEXTURE_TYPES constant below
/// 3. The API will automatically handle it
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum TextureType {
    SKIN,
    CAPE,
    // Add new texture types here, e.g.:
    // ELYTRA,
    // HAT,
}

impl fmt::Display for TextureType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureType::SKIN => write!(f, "SKIN"),
            TextureType::CAPE => write!(f, "CAPE"),
            // Add display for new types here
        }
    }
}

impl std::str::FromStr for TextureType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SKIN" => Ok(TextureType::SKIN),
            "CAPE" => Ok(TextureType::CAPE),
            // Add parsing for new types here
            _ => Err(anyhow::anyhow!(
                "Invalid texture type: {}. Valid types are: {}", 
                s, 
                TextureType::all_types().join(", ")
            )),
        }
    }
}

impl TextureType {
    /// Get all supported texture types
    pub fn all_types() -> Vec<&'static str> {
        vec!["SKIN", "CAPE"] // Add new types here
    }

    /// Get the file extension for this texture type
    pub fn file_extension(&self) -> &str {
        match self {
            TextureType::SKIN => "png",
            TextureType::CAPE => "png",
            // Different types could have different extensions
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextureMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextureResponse {
    pub url: String,
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TextureMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TexturesResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SKIN: Option<TextureResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CAPE: Option<TextureResponse>,
}

#[derive(Debug, FromRow)]
pub struct Texture {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub texture_type: String,
    pub file_hash: String,
    pub file_url: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UploadOptions {
    #[serde(default)]
    pub modelSlim: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub uuid: String,
    pub exp: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
