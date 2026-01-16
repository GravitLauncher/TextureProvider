use crate::auth::AuthUser;
use crate::config::Config;
use crate::models::{TextureMetadata, TextureResponse, TexturesResponse, TextureType, UploadOptions};
use crate::retrieval::TextureRetriever;
use crate::storage::StorageBackend;
use anyhow::Result;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use jsonwebtoken::DecodingKey;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub storage: Arc<dyn StorageBackend>,
    pub retriever: Arc<dyn TextureRetriever>,
    pub public_key : Arc<DecodingKey>,
    pub config: Config,
}

/// GET /get/{uuid} - Get all textures for a user
pub async fn get_textures(
    State(state): State<AppState>,
    Path(user_uuid): Path<Uuid>,
) -> Result<Json<TexturesResponse>, (StatusCode, String)> {
    let mut response = TexturesResponse {
        SKIN: None,
        CAPE: None,
    };

    // Try to get SKIN
    match state.retriever.get_texture(user_uuid, TextureType::SKIN).await {
        Ok(Some(retrieved)) => {
            response.SKIN = Some(TextureResponse {
                url: retrieved.url,
                digest: retrieved.hash,
                metadata: retrieved.metadata,
            });
        }
        Ok(None) => {
            tracing::debug!("No SKIN texture found for user {}", user_uuid);
        }
        Err(e) => {
            tracing::error!("Failed to retrieve SKIN texture: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve SKIN texture: {}", e),
            ));
        }
    }

    // Try to get CAPE
    match state.retriever.get_texture(user_uuid, TextureType::CAPE).await {
        Ok(Some(retrieved)) => {
            response.CAPE = Some(TextureResponse {
                url: retrieved.url,
                digest: retrieved.hash,
                metadata: retrieved.metadata,
            });
        }
        Ok(None) => {
            tracing::debug!("No CAPE texture found for user {}", user_uuid);
        }
        Err(e) => {
            tracing::error!("Failed to retrieve CAPE texture: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve CAPE texture: {}", e),
            ));
        }
    }

    Ok(Json(response))
}

/// GET /get/{uuid}/{texture_type} - Get specific texture
pub async fn get_texture(
    State(state): State<AppState>,
    Path((user_uuid, texture_type_str)): Path<(Uuid, String)>,
) -> Result<Json<TextureResponse>, (StatusCode, String)> {
    let texture_type: TextureType = texture_type_str
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid texture type: {}", e)))?;

    let retrieved = state
        .retriever
        .get_texture(user_uuid, texture_type)
        .await
        .map_err(|e| {
            tracing::error!("Failed to retrieve texture: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve texture: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Texture not found for {}", texture_type_str),
            )
        })?;

    Ok(Json(TextureResponse {
        url: retrieved.url,
        digest: retrieved.hash,
        metadata: retrieved.metadata,
    }))
}

/// POST /upload - Upload a texture file
pub async fn upload_texture(
    State(state): State<AppState>,
    AuthUser(user_uuid): AuthUser,
    Path(texture_type_str): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<TextureResponse>, (StatusCode, String)> {
    let texture_type: TextureType = texture_type_str
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid texture type: {}", e)))?;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut options: Option<UploadOptions> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid multipart data: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read file: {}", e)))?;

                // Validate PNG
                if !is_png(&data) {
                    return Err((StatusCode::BAD_REQUEST, "File must be a PNG image".to_string()));
                }

                file_bytes = Some(data.to_vec());
            }
            "options" => {
                let json_str = field
                    .text()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read options: {}", e)))?;
                options = Some(serde_json::from_str(&json_str).map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Invalid options JSON: {}", e))
                })?);
            }
            _ => {}
        }
    }

    let file_bytes = file_bytes
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "No file provided".to_string()))?;

    let options = options.unwrap_or(UploadOptions { modelSlim: false });

    // Calculate hash
    let hash = state.storage.calculate_hash(&file_bytes);

    // Store file with proper extension
    let file_url = state
        .storage
        .store_file(file_bytes.clone(), &hash, texture_type.file_extension())
        .await
        .map_err(|e| {
            tracing::error!("Failed to store file: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to store file".to_string())
        })?;

    // Prepare metadata
    let metadata = if options.modelSlim {
        Some(serde_json::json!({ "model": "slim" }))
    } else {
        None
    };

    // Insert or update in database
    sqlx::query!(
        r#"
        INSERT INTO textures (user_uuid, texture_type, file_hash, file_url, metadata)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_uuid, texture_type)
        DO UPDATE SET file_hash = $3, file_url = $4, metadata = $5, updated_at = NOW()
        "#,
        user_uuid,
        texture_type.to_string(),
        hash,
        file_url,
        metadata
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to save texture: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save texture".to_string())
    })?;

    Ok(Json(TextureResponse {
        url: file_url,
        digest: hash,
        metadata: if options.modelSlim {
            Some(TextureMetadata {
                model: Some("slim".to_string()),
            })
        } else {
            None
        },
    }))
}

/// GET /download/{texture_type}/{uuid} - Download texture file
pub async fn download_texture(
    State(state): State<AppState>,
    Path((texture_type_str, user_uuid)): Path<(String, Uuid)>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let texture_type: TextureType = texture_type_str
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid texture type: {}", e)))?;

    // Get texture info from database
    let texture = sqlx::query!(
        r#"
        SELECT file_hash
        FROM textures
        WHERE user_uuid = $1 AND texture_type = $2
        "#,
        user_uuid,
        texture_type.to_string()
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch texture: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch texture".to_string())
    })?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Texture not found for {}", texture_type_str),
        )
    })?;

    // Get file bytes from storage
    let file_bytes = state
        .storage
        .get_file(&texture.file_hash, texture_type.file_extension())
        .await
        .map_err(|e| {
            tracing::error!("Failed to get file: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get file".to_string())
        })?;

    Ok((
        [(header::CONTENT_TYPE, "image/png")],
        file_bytes,
    )
        .into_response())
}

/// Check if bytes represent a PNG file
fn is_png(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
}
