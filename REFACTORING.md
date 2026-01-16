# Refactoring Documentation

## Overview

This document describes the major refactoring performed on the texture provider project to improve code organization, remove duplication, and prepare the system for future expansion with new texture types.

## Key Changes

### 1. Storage Architecture (Trait-Based Design)

**Before:** Single `Storage` struct with runtime dispatching
**After:** Trait-based abstraction with separate implementations

#### New File Structure
```
src/storage/
├── mod.rs          # Module exports and factory function
├── backend.rs      # StorageBackend trait definition
├── local.rs        # LocalStorage implementation
└── s3.rs           # S3Storage implementation
```

#### Benefits
- **Separation of Concerns**: Each storage type has its own file
- **No Duplicate Code**: Removed duplicate S3 client initialization
- **Extensibility**: Easy to add new storage backends (e.g., Azure, GCS)
- **Type Safety**: Compile-time guarantees through trait system
- **Testability**: Each backend can be tested independently

### 2. Texture Type System

**Before:** Hardcoded in multiple places
**After:** Centralized enum with helper methods

#### Enhanced TextureType Enum
```rust
pub enum TextureType {
    SKIN,
    CAPE,
    // Add new types here
}
```

#### Key Features
- `Display` trait for string conversion
- `FromStr` trait for parsing
- `file_extension()` method for type-specific extensions
- `all_types()` method to list supported types
- Detailed error messages showing valid types

### 3. Storage Backend Interface

The `StorageBackend` trait provides a clean interface:

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store_file(&self, bytes: Vec<u8>, hash: &str, extension: &str) -> Result<String>;
    async fn get_file(&self, hash: &str, extension: &str) -> Result<Vec<u8>>;
    fn generate_url(&self, hash: &str, extension: &str) -> String;
    fn calculate_hash(&self, bytes: &[u8]) -> String;
}
```

## How to Add a New Texture Type

Adding a new texture type (e.g., `ELYTRA`) is now a simple 3-step process:

### Step 1: Add to TextureType Enum

Edit `src/models.rs`:

```rust
pub enum TextureType {
    SKIN,
    CAPE,
    ELYTRA,  // Add this
}
```

### Step 2: Update Display Implementation

```rust
impl fmt::Display for TextureType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureType::SKIN => write!(f, "SKIN"),
            TextureType::CAPE => write!(f, "CAPE"),
            TextureType::ELYTRA => write!(f, "ELYTRA"),  // Add this
        }
    }
}
```

### Step 3: Update FromStr Implementation

```rust
impl std::str::FromStr for TextureType {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SKIN" => Ok(TextureType::SKIN),
            "CAPE" => Ok(TextureType::CAPE),
            "ELYTRA" => Ok(TextureType::ELYTRA),  // Add this
            _ => Err(anyhow::anyhow!("Invalid texture type: {}", s)),
        }
    }
}
```

### Step 4: Update Helper Methods

```rust
impl TextureType {
    pub fn all_types() -> Vec<&'static str> {
        vec!["SKIN", "CAPE", "ELYTRA"]  // Add "ELYTRA"
    }

    pub fn file_extension(&self) -> &str {
        match self {
            TextureType::SKIN => "png",
            TextureType::CAPE => "png",
            TextureType::ELYTRA => "png",  // Add this (could be different)
        }
    }
}
```

### Step 5: Update Response Struct (Optional)

If you want the new type in the combined response:

Edit `src/models.rs`:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct TexturesResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SKIN: Option<TextureResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CAPE: Option<TextureResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ELYTRA: Option<TextureResponse>,  // Add this
}
```

And update the handler in `src/handlers.rs`:

```rust
let mut response = TexturesResponse {
    SKIN: None,
    CAPE: None,
    ELYTRA: None,  // Add this
};

// In the match statement:
match texture.texture_type.as_str() {
    "SKIN" => response.SKIN = Some(texture_response),
    "CAPE" => response.CAPE = Some(texture_response),
    "ELYTRA" => response.ELYTRA = Some(texture_response),  // Add this
    _ => {}
}
```

**That's it!** The API will now automatically handle the new texture type for:
- Uploading (`POST /upload/ELYTRA`)
- Retrieving single (`GET /get/{uuid}/ELYTRA`)
- Retrieving all (`GET /get/{uuid}`)
- Downloading (`GET /download/ELYTRA/{uuid}`)

## How to Add a New Storage Backend

Adding a new storage backend (e.g., Azure Blob Storage) follows the same pattern:

### Step 1: Implement the Trait

Create `src/storage/azure.rs`:

```rust
use super::backend::StorageBackend;
use crate::config::Config;
use anyhow::Result;
use async_trait::async_trait;

pub struct AzureStorage {
    container_name: String,
    // Add Azure-specific fields
}

impl AzureStorage {
    pub fn new(config: Config) -> Self {
        // Initialize Azure client
    }
}

#[async_trait]
impl StorageBackend for AzureStorage {
    async fn store_file(&self, bytes: Vec<u8>, hash: &str, extension: &str) -> Result<String> {
        // Implement Azure storage logic
    }

    async fn get_file(&self, hash: &str, extension: &str) -> Result<Vec<u8>> {
        // Implement Azure retrieval logic
    }

    fn generate_url(&self, hash: &str, extension: &str) -> String {
        // Implement Azure URL generation
    }
}
```

### Step 2: Update Factory Function

Edit `src/storage/mod.rs`:

```rust
pub mod backend;
pub mod local;
pub mod s3;
pub mod azure;  // Add this

pub use backend::StorageBackend;
pub use local::LocalStorage;
pub use s3::S3Storage;
pub use azure::AzureStorage;  // Add this

pub fn create_storage(config: Config) -> Arc<dyn StorageBackend> {
    match config.storage_type {
        crate::config::StorageType::Local => Arc::new(LocalStorage::new(config)),
        crate::config::StorageType::S3 => Arc::new(S3Storage::new(config)),
        crate::config::StorageType::Azure => Arc::new(AzureStorage::new(config)),  // Add this
    }
}
```

### Step 3: Add to Config

Edit `src/config.rs` to add Azure variant to `StorageType` enum and configuration fields.

## Testing Strategy

### Unit Testing

Each storage backend can be tested independently:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_storage() {
        let storage = LocalStorage::new(test_config());
        // Test store_file, get_file, generate_url
    }
}
```

### Integration Testing

The application can be tested with different storage backends by setting environment variables.

## Performance Improvements

1. **Removed Duplicate S3 Client Creation**: Client is now created once per backend instance
2. **Better Resource Management**: Each backend manages its own resources
3. **Cleaner Error Handling**: Specific error messages for each backend type

## Migration Notes

### Breaking Changes

None! The API endpoints remain exactly the same. The refactoring is entirely internal.

### Configuration

No configuration changes needed. The existing `.env` variables work as before.

## 4. Texture Retrieval Chain (Chain of Responsibility)

**Latest Refactoring (January 2026)**

### Overview

Implemented a chain-of-responsibility pattern for texture acquisition, allowing multiple retrieval strategies to be tried in sequence. The first handler to successfully return a texture will be used.

### Before
- Single retriever type (Storage, Mojang, or Default Skin)
- Configured via `RETRIEVAL_TYPE` environment variable
- Only one retrieval strategy could be active at a time

### After
- Chain of retrievers that can be configured in any order
- Multiple retrieval strategies can be tried in sequence
- Configured via `RETRIEVAL_CHAIN` environment variable (comma-separated)
- Backward compatible: `RETRIEVAL_TYPE` still works as before

### New File Structure
```
src/retrieval/
├── mod.rs              # Module exports, factory function, chain builder
├── backend.rs          # TextureRetriever trait definition
├── chain.rs            # NEW: ChainRetriever implementation
├── storage_retriever.rs
├── mojang.rs
└── default_skin.rs
```

### ChainRetriever Implementation

The `ChainRetriever` implements the `TextureRetriever` trait and manages multiple handlers:

```rust
pub struct ChainRetriever {
    handlers: Vec<Arc<dyn TextureRetriever>>,
}

impl ChainRetriever {
    /// Create a new chain with handlers in order
    pub fn new(handlers: Vec<Arc<dyn TextureRetriever>>) -> Self;
}
```

#### Key Features
- **Ordered Execution**: Handlers are tried in the specified order
- **First Successful**: Returns immediately when a handler finds a texture
- **Error Resilience**: Continues to next handler on errors
- **Type Filtering**: Skips handlers that don't support the requested texture type
- **Comprehensive Logging**: Debug logs for each handler attempt

### Configuration

#### Environment Variables

**Single Retriever (Legacy)**:
```bash
RETRIEVAL_TYPE=storage  # Options: storage, mojang, default_skin
```

**Chain Retrievers (New)**:
```bash
RETRIEVAL_CHAIN=storage,mojang,default_skin
```

#### Example Configurations

1. **Try storage first, fallback to Mojang API**:
   ```bash
   RETRIEVAL_CHAIN=storage,mojang
   ```

2. **Try Mojang first, fallback to storage, then default skin**:
   ```bash
   RETRIEVAL_CHAIN=mojang,storage,default_skin
   ```

3. **Only use default skins**:
   ```bash
   RETRIEVAL_CHAIN=default_skin
   ```

### Benefits

1. **Flexible Fallback**: Multiple retrieval strategies can coexist
2. **Graceful Degradation**: System continues working even if one source fails
3. **Easy to Extend**: Add new retrievers without modifying existing code
4. **Backward Compatible**: Existing single-retriever configurations still work
5. **Performance**: Short-circuits on first success, no unnecessary calls
6. **Observable**: Detailed logging for debugging and monitoring

### How the Chain Works

For a request like `GET /get/{uuid}/SKIN`:

```
Handler 1 (Storage) → Found? Return texture
                   ↓ Not Found
Handler 2 (Mojang)  → Found? Return texture
                   ↓ Not Found/Error
Handler 3 (Default) → Return default Steve skin
```

### Adding a New Retriever to the Chain

#### Step 1: Implement the Trait

Create `src/retrieval/custom.rs`:

```rust
use super::backend::{RetrievedTexture, TextureRetriever};
use crate::models::{TextureType};
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

pub struct CustomRetriever {
    // Add custom fields
}

impl CustomRetriever {
    pub fn new() -> Self {
        // Initialize
    }
}

#[async_trait]
impl TextureRetriever for CustomRetriever {
    async fn get_texture(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>> {
        // Implement retrieval logic
    }

    fn supports_texture_type(&self, texture_type: TextureType) -> bool {
        // Return which types this retriever supports
        matches!(texture_type, TextureType::SKIN)
    }
}
```

#### Step 2: Add to Config

Edit `src/config.rs`:

```rust
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum RetrievalType {
    Storage,
    Mojang,
    DefaultSkin,
    Custom,  // Add this
}

impl std::str::FromStr for RetrievalType {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "storage" => Ok(RetrievalType::Storage),
            "mojang" => Ok(RetrievalType::Mojang),
            "default_skin" => Ok(RetrievalType::DefaultSkin),
            "custom" => Ok(RetrievalType::Custom),  // Add this
            _ => Err(anyhow::anyhow!("Invalid retrieval type: {}", s)),
        }
    }
}
```

#### Step 3: Update Factory Function

Edit `src/retrieval/mod.rs`:

```rust
pub mod custom;  // Add this

pub use custom::CustomRetriever;  // Add this

fn create_retriever_by_type(
    retrieval_type: &RetrievalType,
    config: &Config,
    storage: Arc<dyn crate::storage::StorageBackend>,
    db: sqlx::PgPool,
) -> Arc<dyn TextureRetriever> {
    match retrieval_type {
        RetrievalType::Storage => Arc::new(StorageRetriever::new(storage, db)),
        RetrievalType::Mojang => Arc::new(MojangRetriever::new(config.clone())),
        RetrievalType::DefaultSkin => Arc::new(DefaultSkinRetriever::new()),
        RetrievalType::Custom => Arc::new(CustomRetriever::new()),  // Add this
    }
}
```

#### Step 4: Configure the Chain

```bash
RETRIEVAL_CHAIN=storage,custom,mojang,default_skin
```

### Testing

The chain retriever includes comprehensive unit tests:

```bash
cargo test retrieval::chain::tests
```

Tests cover:
- ✅ First successful result returned
- ✅ Chain continues on handler errors
- ✅ Returns None if all handlers fail
- ✅ Skips handlers that don't support the texture type

### Migration Notes

#### No Breaking Changes!
- Existing `RETRIEVAL_TYPE` configuration continues to work
- API endpoints unchanged
- Default behavior unchanged (if `RETRIEVAL_CHAIN` not set)

#### Recommended Migration

For most users, a sensible chain is:

```bash
RETRIEVAL_CHAIN=storage,mojang,default_skin
```

This provides:
1. Fast local lookup (storage)
2. Fallback to official Mojang API (mojang)
3. Final fallback to default skin (default_skin)

### Logging

The chain provides detailed logging for observability:

```
INFO  Creating retrieval chain with 3 handlers: [Storage, Mojang, DefaultSkin]
DEBUG Handler 0 does not support texture type CAPE, skipping
DEBUG Trying handler 1 for texture type SKIN
DEBUG Handler 1 successfully retrieved texture for user 123e4567-e89b-12d3-a456-426614174000
WARN  Handler 0 failed with error: Connection timeout, trying next handler
DEBUG Handler 1 found no texture for user 123e4567-e89b-12d3-a456-426614174000, trying next handler
```

## Future Enhancements

Possible future improvements:

1. **Storage Adapter Pattern**: Add caching layer
2. **Multi-Storage Support**: Store in multiple backends simultaneously
3. **Storage Health Checks**: Monitor storage backend health
4. **Retrieval Chain Caching**: Cache successful retrievals in the chain
5. **Migration Tools**: Move data between storage backends
6. **Compression**: Add transparent compression support
7. **CDN Integration**: Automatic CDN URL generation
8. **Metrics Collection**: Track retrieval success rates per handler
