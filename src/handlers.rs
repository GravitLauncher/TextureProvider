use crate::auth::{AuthAdmin, AuthUser};
use crate::config::Config;
use crate::models::{
    TextureMetadata, TextureResponse, TextureType, TexturesResponse, UploadOptions,
};
use crate::retrieval::{download_file_from_url, TextureRetriever};
use crate::storage::StorageBackend;
use anyhow::{anyhow, Result};
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

/// Maximum file size for uploads (1 MB)
/// PNG texture files for Minecraft skins/capes should never exceed this
const MAX_FILE_SIZE: usize = 1_048_576; // 1 MB in bytes

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub storage: Arc<dyn StorageBackend>,
    pub retriever: Arc<dyn TextureRetriever>,
    pub public_key: Arc<DecodingKey>,
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

    // Use the retriever's get_textures method to retrieve all textures at once
    let textures = state
        .retriever
        .get_textures(user_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to retrieve textures: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve textures: {}", e),
            )
        })?;

    // Extract SKIN if available
    if let Some(retrieved) = textures.get("SKIN") {
        response.SKIN = Some(TextureResponse {
            url: retrieved.url.clone(),
            digest: retrieved.hash.clone(),
            metadata: retrieved.metadata.clone(),
        });
    } else {
        tracing::debug!("No SKIN texture found for user {}", user_uuid);
    }

    // Extract CAPE if available
    if let Some(retrieved) = textures.get("CAPE") {
        response.CAPE = Some(TextureResponse {
            url: retrieved.url.clone(),
            digest: retrieved.hash.clone(),
            metadata: retrieved.metadata.clone(),
        });
    } else {
        tracing::debug!("No CAPE texture found for user {}", user_uuid);
    }

    Ok(Json(response))
}

/// GET /get/{uuid}/{texture_type} - Get specific texture
pub async fn get_texture(
    State(state): State<AppState>,
    Path((user_uuid, texture_type_str)): Path<(Uuid, String)>,
) -> Result<Json<TextureResponse>, (StatusCode, String)> {
    let texture_type: TextureType = texture_type_str.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid texture type: {}", e),
        )
    })?;

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
    let texture_type: TextureType = texture_type_str.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid texture type: {}", e),
        )
    })?;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut options: Option<UploadOptions> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid multipart data: {}", e),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let data = field.bytes().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read file: {}", e),
                    )
                })?;

                // Validate file size
                if data.len() > MAX_FILE_SIZE {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "File size {} bytes exceeds maximum allowed size of {} bytes (1 MB)",
                            data.len(),
                            MAX_FILE_SIZE
                        ),
                    ));
                }

                // Validate PNG
                if !is_png(&data) {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "File must be a PNG image".to_string(),
                    ));
                }

                file_bytes = Some(data.to_vec());
            }
            "options" => {
                let json_str = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read options: {}", e),
                    )
                })?;
                options = Some(serde_json::from_str(&json_str).map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid options JSON: {}", e),
                    )
                })?);
            }
            _ => {}
        }
    }

    let file_bytes =
        file_bytes.ok_or_else(|| (StatusCode::BAD_REQUEST, "No file provided".to_string()))?;

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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to store file".to_string(),
            )
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
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to save texture".to_string(),
        )
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
    let texture_type: TextureType = texture_type_str.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid texture type: {}", e),
        )
    })?;

    // Use the retriever to get texture bytes (efficient, no duplication)
    let retrieved = state
        .retriever
        .get_texture_bytes(user_uuid, texture_type)
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

    Ok(([(header::CONTENT_TYPE, "image/png")], retrieved.bytes).into_response())
}

/// GET /files/{hash}.{ext} - Serve texture files directly from storage
/// This provides efficient file distribution for files that have been uploaded
pub async fn serve_texture_file(
    State(state): State<AppState>,
    Path((hash)): Path<(String)>,
) -> Result<Response<Body>, (StatusCode, String)> {
    // Get file bytes from storage by hash
    let file_bytes = state.storage.get_file(&hash, ".png").await.map_err(|e| {
        tracing::error!("Failed to get file: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to get file".to_string(),
        )
    })?;

    Ok(([(header::CONTENT_TYPE, "image/png")], file_bytes).into_response())
}

/// Check if bytes represent a PNG file
fn is_png(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
}

/// POST /api/upload/:type - Upload a texture for any user (admin only)
/// Requires admin bearer token. User UUID is provided in the "user" form field.
pub async fn admin_upload_texture(
    State(state): State<AppState>,
    AuthAdmin: AuthAdmin,
    Path(texture_type_str): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<TextureResponse>, (StatusCode, String)> {
    let texture_type: TextureType = texture_type_str.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid texture type: {}", e),
        )
    })?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut options: Option<UploadOptions> = None;
    let mut user_uuid: Option<Uuid> = None;
    let mut user_username: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid multipart data: {}", e),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let data = field.bytes().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read file: {}", e),
                    )
                })?;

                // Validate file size
                if data.len() > MAX_FILE_SIZE {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "File size {} bytes exceeds maximum allowed size of {} bytes (1 MB)",
                            data.len(),
                            MAX_FILE_SIZE
                        ),
                    ));
                }

                // Validate PNG
                if !is_png(&data) {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "File must be a PNG image".to_string(),
                    ));
                }

                file_bytes = Some(data.to_vec());
            }
            "options" => {
                let json_str = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read options: {}", e),
                    )
                })?;
                options = Some(serde_json::from_str(&json_str).map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid options JSON: {}", e),
                    )
                })?);
            }
            "uuid" => {
                let uuid_str = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read user UUID: {}", e),
                    )
                })?;
                user_uuid =
                    Some(Uuid::parse_str(&uuid_str).map_err(|e| {
                        (StatusCode::BAD_REQUEST, format!("Invalid user UUID: {}", e))
                    })?);
            }
            "username" => {
                let username_str = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read user UUID: {}", e),
                    )
                })?;
                user_username = Some(username_str);
            }
            _ => {}
        }
    }

    let user_uuid = user_uuid.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "User UUID not provided".to_string(),
        )
    })?;

    if let Some(username) = user_username {
        sqlx::query!(
            r#"
        INSERT INTO username_mappings (user_uuid, username, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (user_uuid, username)
        DO UPDATE SET updated_at = NOW()
        "#,
            user_uuid,
            username
        )
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update username mapping: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update username mapping".to_string(),
            )
        })?;
    }

    let file_bytes =
        file_bytes.ok_or_else(|| (StatusCode::BAD_REQUEST, "No file provided".to_string()))?;

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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to store file".to_string(),
            )
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
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to save texture".to_string(),
        )
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

/// GET /download/:hash - Download skin by hash
/// Uses the retrieval chain to get texture bytes by hash (StorageRetriever, EmbeddedDefaultSkinRetriever, etc.)
/// Falls back to http/https download if the texture has an external URL in the database
pub async fn download_by_hash(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let cache_max_age = state.config.hash_cache_seconds;
    let cache_control = format!("public, max-age={}", cache_max_age);
    // Try to get from retriever chain by hash
    // The chain will try StorageRetriever (handles both S3 and local storage),
    // then EmbeddedDefaultSkinRetriever, then other retrievers in order
    match state.retriever.get_texture_bytes_by_hash(&hash).await {
        Ok(Some(retrieved)) => {
            return Ok((
                [
                    (header::CONTENT_TYPE, "image/png"),
                    (header::CACHE_CONTROL, cache_control.as_str()),
                ],
                retrieved.bytes,
            )
                .into_response());
        }
        Ok(None) => {
            tracing::debug!("Retriever chain did not provide texture for hash: {}", hash);
        }
        Err(e) => {
            tracing::warn!("Retriever chain error for hash {}: {}", hash, e);
        }
    }

    // Check database for a texture with this hash to potentially fetch from external URL
    // This handles cases where textures are stored with http/https URLs (e.g., Mojang API URLs)
    let texture_record = sqlx::query!(
        r#"
        SELECT file_url
        FROM textures
        WHERE file_hash = $1
        LIMIT 1
        "#,
        hash
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query database: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database query failed".to_string(),
        )
    })?;

    if let Some(record) = texture_record {
        // If we have a record with an http/https URL, try to fetch from there
        if record.file_url.starts_with("http://") || record.file_url.starts_with("https://") {
            tracing::debug!("Attempting to fetch texture from URL: {}", record.file_url);

            match download_file_from_url(&record.file_url).await {
                Ok(Some(bytes)) => {
                    return Ok((
                        [
                            (header::CONTENT_TYPE, "image/png"),
                            (header::CACHE_CONTROL, cache_control.as_str()),
                        ],
                        bytes,
                    )
                        .into_response());
                }
                Ok(None) => {
                    tracing::warn!("Failed to download texture from URL: {}", record.file_url);
                }
                Err(err) => {
                    tracing::error!("Error downloading texture from URL: {}", err);
                }
            }
        }
    }

    // If all attempts fail, return 404
    Err((
        StatusCode::NOT_FOUND,
        format!("Texture not found for hash: {}", hash),
    ))
}

/// GET /api/get/:username/:uuid - Get all textures for a user by username/uuid (admin only)
/// This endpoint requires an admin token and will update the username<->uuid mapping
/// Returns the same content as /get/:uuid but updates the unreliable username mapping
pub async fn get_textures_by_username_uuid(
    State(state): State<AppState>,
    AuthAdmin: AuthAdmin,
    Path((username, user_uuid)): Path<(String, Uuid)>,
) -> Result<Json<TexturesResponse>, (StatusCode, String)> {
    // Update or insert the username<->uuid mapping
    sqlx::query!(
        r#"
        INSERT INTO username_mappings (user_uuid, username, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (user_uuid, username)
        DO UPDATE SET updated_at = NOW()
        "#,
        user_uuid,
        username
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update username mapping: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to update username mapping".to_string(),
        )
    })?;

    tracing::info!("Updated username mapping: {} <-> {}", username, user_uuid);

    // Now get the textures using the UUID (reuse existing logic)
    let textures = state
        .retriever
        .get_textures(user_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to retrieve textures: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve textures: {}", e),
            )
        })?;

    let mut response = TexturesResponse {
        SKIN: None,
        CAPE: None,
    };

    // Extract SKIN if available
    if let Some(retrieved) = textures.get("SKIN") {
        response.SKIN = Some(TextureResponse {
            url: retrieved.url.clone(),
            digest: retrieved.hash.clone(),
            metadata: retrieved.metadata.clone(),
        });
    } else {
        tracing::debug!("No SKIN texture found for user {}", user_uuid);
    }

    // Extract CAPE if available
    if let Some(retrieved) = textures.get("CAPE") {
        response.CAPE = Some(TextureResponse {
            url: retrieved.url.clone(),
            digest: retrieved.hash.clone(),
            metadata: retrieved.metadata.clone(),
        });
    } else {
        tracing::debug!("No CAPE texture found for user {}", user_uuid);
    }

    Ok(Json(response))
}

/// GET /download/username/:texture_type/:username - Download texture by username
/// This endpoint looks up the UUID from username and returns the texture with cache headers
/// Cache lifetime is configurable via USERNAME_CACHE_SECONDS (default 8 hours)
///
/// Flow:
/// 1. Try to find username in local mappings
/// 2. If not found, use the retrieval chain which may include Mojang API resolution
/// 3. Save the new mapping if chain successfully resolved it
/// 4. Return the texture with cache headers
pub async fn download_texture_by_username(
    State(state): State<AppState>,
    Path((texture_type_str, username)): Path<(String, String)>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let texture_type: TextureType = texture_type_str.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid texture type: {}", e),
        )
    })?;

    // Try to look up the UUID from username in local database first
    let user_uuid = match sqlx::query!(
        r#"
        SELECT user_uuid
        FROM username_mappings
        WHERE username = $1
        LIMIT 1
        "#,
        username
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(result)) => {
            tracing::debug!(
                "Resolved username {} to UUID {} from local mapping",
                username,
                result.user_uuid
            );
            Some(result.user_uuid)
        }
        Ok(None) => {
            tracing::debug!("Username {} not found in local mappings", username);
            None
        }
        Err(e) => {
            tracing::error!("Failed to lookup username: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to lookup username".to_string(),
            ));
        }
    };

    // If we have a local mapping, use it directly
    let retrieved = if let Some(uuid) = user_uuid {
        // Use the retriever chain with the UUID
        state
            .retriever
            .get_texture_bytes(uuid, texture_type)
            .await
            .map_err(|e| {
                tracing::error!("Failed to retrieve texture: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to retrieve texture: {}", e),
                )
            })?
            .ok_or_else(|| {
                tracing::debug!("Texture not found for {} {}", texture_type_str, uuid);
                (
                    StatusCode::NOT_FOUND,
                    format!("Texture not found for {}", texture_type_str),
                )
            })?
    } else {
        // No local mapping, try the retrieval chain with username
        // The chain may include MojangRetriever which can resolve usernames
        tracing::info!(
            "Attempting to retrieve texture for username {} via retrieval chain",
            username
        );

        match state
            .retriever
            .get_texture_bytes_by_username(&username, texture_type)
            .await
        {
            Ok(Some(texture_bytes)) => {
                tracing::info!(
                    "Successfully retrieved texture for username {} via retrieval chain",
                    username
                );

                // If the retrieval succeeded, we might have resolved a UUID
                // Try to save the mapping if we can extract it (optional optimization)
                // For now, just return the texture
                texture_bytes
            }
            Ok(None) => {
                tracing::debug!(
                    "Retrieval chain could not find texture for username {}",
                    username
                );
                return Err((
                    StatusCode::NOT_FOUND,
                    format!("Username '{}' not found", username),
                ));
            }
            Err(e) => {
                tracing::error!("Failed to retrieve texture via chain: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to retrieve texture: {}", e),
                ));
            }
        }
    };

    // Calculate cache max-age from config
    let cache_max_age = state.config.username_cache_seconds;
    let cache_control = format!("private, max-age={}", cache_max_age);

    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, cache_control.as_str()),
        ],
        retrieved.bytes,
    )
        .into_response())
}
