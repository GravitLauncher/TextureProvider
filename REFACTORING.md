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

## Future Enhancements

Possible future improvements:

1. **Storage Adapter Pattern**: Add caching layer
2. **Multi-Storage Support**: Store in multiple backends simultaneously
3. **Storage Health Checks**: Monitor storage backend health
4. **Migration Tools**: Move data between storage backends
5. **Compression**: Add transparent compression support
6. **CDN Integration**: Automatic CDN URL generation
