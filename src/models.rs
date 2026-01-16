use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TextureType {
    SKIN,
    CAPE,
}

impl ToString for TextureType {
    fn to_string(&self) -> String {
        match self {
            TextureType::SKIN => "SKIN".to_string(),
            TextureType::CAPE => "CAPE".to_string(),
        }
    }
}

impl std::str::FromStr for TextureType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SKIN" => Ok(TextureType::SKIN),
            "CAPE" => Ok(TextureType::CAPE),
            _ => Err(anyhow::anyhow!("Invalid texture type: {}", s)),
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
