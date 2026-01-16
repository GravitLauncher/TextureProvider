# Texture Retrieval Architecture

## Overview

The texture provider has been refactored to separate **texture storage** from **texture retrieval**. This allows for multiple retrieval strategies while keeping a consistent storage backend for uploads.

## Architecture

### Separation of Concerns

- **Storage**: Handles how texture files are stored (Local, S3, etc.)
- **Retrieval**: Handles how textures are fetched (Storage, Mojang API, Default Skins)

### Components

```
src/
├── storage/          # Storage backends (for uploads)
│   ├── backend.rs    # StorageBackend trait
│   ├── local.rs      # Local file storage
│   └── s3.rs         # S3 storage
└── retrieval/        # Retrieval strategies (for downloads)
    ├── backend.rs    # TextureRetriever trait
    ├── storage_retriever.rs    # Retrieve from storage
    ├── mojang.rs     # Retrieve from Mojang API
    └── default_skin.rs         # Return default skins
```

## Retrieval Strategies

### 1. Storage Retrieval (Default)

Retrieves textures that were uploaded by users and stored in your storage backend (Local or S3).

```env
RETRIEVAL_TYPE=storage
```

**Use cases:**
- User-uploaded custom skins
- User-uploaded capes
- When you want full control over textures

**How it works:**
- Queries database for texture metadata
- Returns stored texture URLs and hashes
- Original behavior before refactoring

### 2. Mojang API Retrieval

Fetches official Minecraft skins and capes directly from Mojang's session servers.

```env
RETRIEVAL_TYPE=mojang
```

**Use cases:**
- Displaying official player skins
- Showing capes for players who have them
- No need for users to upload textures

**How it works:**
1. Receives player UUID
2. Fetches profile from Mojang session server
3. Decodes base64-encoded textures
4. Returns texture URLs (hosted on textures.minecraft.net)

**API Endpoints used:**
- `https://sessionserver.mojang.com/session/minecraft/profile/{uuid}`

**Note:** 
- Returns `None` for users without custom capes
- Includes skin metadata (slim vs. classic model)
- Requires internet connection to Mojang API

### 3. Default Skin Retrieval

Returns the default Steve skin for all users who don't have a custom skin.

```env
RETRIEVAL_TYPE=default_skin
```

**Use cases:**
- Fallback for users without skins
- Testing environments
- When you want a consistent appearance

**How it works:**
- Returns official default Steve skin URL
- Returns `None` for capes (default capes don't exist)
- No database or API calls needed

## Configuration

Add to your `.env` file:

```env
# Choose retrieval strategy: storage, mojang, or default_skin
RETRIEVAL_TYPE=storage

# Storage configuration (still required for uploads)
STORAGE_TYPE=local
LOCAL_STORAGE_PATH=./uploads
```

## API Behavior

### GET /get/{uuid}

Retrieves all textures for a user. Behavior depends on retrieval type:

**Storage Mode:**
```json
{
  "SKIN": {
    "url": "http://localhost:3000/uploads/abc123.png",
    "digest": "abc123...",
    "metadata": null
  },
  "CAPE": {
    "url": "http://localhost:3000/uploads/def456.png",
    "digest": "def456...",
    "metadata": null
  }
}
```

**Mojang Mode:**
```json
{
  "SKIN": {
    "url": "http://textures.minecraft.net/texture/...",
    "digest": "...",
    "metadata": {
      "model": "slim"
    }
  },
  "CAPE": null  // or texture data if player has cape
}
```

**Default Skin Mode:**
```json
{
  "SKIN": {
    "url": "http://textures.minecraft.net/texture/1a4af718...",
    "digest": "1a4af718...",
    "metadata": null
  },
  "CAPE": null
}
```

### POST /upload

Upload functionality is **unchanged** - uploads always use the configured storage backend.

## Extending Retrieval

To add a new retrieval method:

### 1. Create the Retriever

Create `src/retrieval/my_method.rs`:

```rust
use super::backend::{RetrievedTexture, TextureRetriever};
use crate::models::{TextureMetadata, TextureType};
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

pub struct MyRetriever {
    // Add fields
}

impl MyRetriever {
    pub fn new() -> Self {
        MyRetriever {
            // Initialize
        }
    }
}

#[async_trait]
impl TextureRetriever for MyRetriever {
    async fn get_texture(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>> {
        // Implement retrieval logic
        Ok(Some(RetrievedTexture {
            url: "...".to_string(),
            hash: "...".to_string(),
            metadata: None,
        }))
    }

    fn supports_texture_type(&self, texture_type: TextureType) -> bool {
        // Return which texture types are supported
        matches!(texture_type, TextureType::SKIN | TextureType::CAPE)
    }
}
```

### 2. Add to Module

Edit `src/retrieval/mod.rs`:

```rust
pub mod my_method;

pub use my_method::MyRetriever;

pub fn create_retriever(
    config: Config,
    storage: Arc<dyn crate::storage::StorageBackend>,
    db: sqlx::PgPool,
) -> Arc<dyn TextureRetriever> {
    match config.retrieval_type {
        crate::config::RetrievalType::Storage => {
            Arc::new(StorageRetriever::new(storage, db))
        }
        crate::config::RetrievalType::Mojang => {
            Arc::new(MojangRetriever::new(config))
        }
        crate::config::RetrievalType::DefaultSkin => {
            Arc::new(DefaultSkinRetriever::new())
        }
        crate::config::RetrievalType::MyMethod => {
            Arc::new(MyRetriever::new())
        }
    }
}
```

### 3. Add Configuration Type

Edit `src/config.rs`:

```rust
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum RetrievalType {
    Storage,
    Mojang,
    DefaultSkin,
    MyMethod,  // Add this
}

impl std::str::FromStr for RetrievalType {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "storage" => Ok(RetrievalType::Storage),
            "mojang" => Ok(RetrievalType::Mojang),
            "default_skin" => Ok(RetrievalType::DefaultSkin),
            "my_method" => Ok(RetrievalType::MyMethod),  // Add this
            _ => Err(anyhow::anyhow!("Invalid retrieval type: {}", s)),
        }
    }
}
```

### 4. Update .env.example

```env
# Options: storage, mojang, default_skin, my_method
RETRIEVAL_TYPE=storage
```

## Testing Different Retrieval Methods

### Test Storage Retrieval
```bash
# Start server
RETRIEVAL_TYPE=storage cargo run

# Upload a skin
curl -X POST http://localhost:3000/upload/SKIN \
  -H "Authorization: Bearer <JWT>" \
  -F "file=@skin.png"

# Get textures
curl http://localhost:3000/get/<UUID>
```

### Test Mojang Retrieval
```bash
# Start server
RETRIEVAL_TYPE=mojang cargo run

# Get textures (no upload needed)
curl http://localhost:3000/get/<UUID>

# Example: Get Notch's skin
curl http://localhost:3000/get/069a79f4-44e9-4726-a5be-fca90e38aaf5
```

### Test Default Skin Retrieval
```bash
# Start server
RETRIEVAL_TYPE=default_skin cargo run

# Get textures (any UUID will return Steve skin)
curl http://localhost:3000/get/<any-uuid>
```

## Benefits of This Architecture

1. **Separation of Concerns**: Storage and retrieval are independent
2. **Flexibility**: Easy to switch between retrieval methods
3. **Extensibility**: Simple to add new retrieval strategies
4. **Backwards Compatible**: Storage-based retrieval maintains original behavior
5. **Testability**: Each retriever can be tested independently
6. **Performance**: Can optimize retrieval without affecting storage

## Migration Guide

If you're upgrading from the old version:

1. **No database changes needed** - existing uploads continue to work
2. **Set `RETRIEVAL_TYPE=storage`** to maintain current behavior
3. **Upload flow unchanged** - users can still upload textures
4. **Optional**: Switch to Mojang or Default Skin retrieval as needed

## Performance Considerations

### Storage Retrieval
- ✅ Fast: Direct database + storage access
- ✅ Reliable: No external dependencies
- ❌ Requires uploads

### Mojang Retrieval
- ✅ No uploads needed
- ✅ Always shows official skins
- ❌ External API dependency
- ❌ Network latency
- ❌ Rate limiting (Mojang API)

### Default Skin Retrieval
- ✅ Fastest: No DB or API calls
- ✅ No dependencies
- ❌ No customization
- ❌ Cape support (returns None)

## Future Enhancements

Possible improvements:

1. **Caching**: Cache Mojang API responses
2. **Fallback Chain**: Try Mojang, fall back to default
3. **Mixed Mode**: Use storage for some users, Mojang for others
4. **CDN Integration**: Cache textures on CDN
5. **WebSocket Updates**: Push texture updates to clients
