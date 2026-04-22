# Texture Provider Service

A high-performance RESTful web service for managing Minecraft texture files (SKIN and CAPE) with flexible storage and retrieval strategies.

## Features

- Upload and retrieve PNG texture files with SHA256 hash-based naming
- JWT-based authentication using ES256 algorithm (compatible with LaunchServer)
- Flexible storage backends: Local filesystem or S3-compatible storage
- Flexible retrieval strategies with chain support:
  - Storage: Retrieve from local database/storage
  - Mojang: Fallback to official Mojang API
  - DefaultSkin: Generate default Steve/Alex skins
  - Chain: Combine multiple strategies with fallback logic
- PostgreSQL database for metadata and caching
- Username-based texture lookup with configurable caching
- Admin API with token-based authentication
- CORS support with configurable origins
- Axum web framework for high performance
- Automatic metadata support (e.g., slim model skins)

## Environment Variables

Copy `.env.example` to `.env` and configure:

```bash
# Required
DATABASE_URL=postgresql://username:password@localhost/texture_provider
JWT_PUBLIC_KEY=BASE64_ECDSA_LAUNCHSERVER_KEY

# Optional (defaults shown)
BASE_URL=http://localhost:3000
SERVER_PORT=3000

# Storage Configuration
STORAGE_TYPE=local                    # Options: local, s3
LOCAL_STORAGE_PATH=./uploads          # Required if STORAGE_TYPE=local

# S3 Storage (required if STORAGE_TYPE=s3)
S3_BUCKET=your-bucket-name
S3_REGION=us-east-1
S3_ENDPOINT=https://s3.amazonaws.com
S3_ACCESS_KEY=your-access-key
S3_SECRET_KEY=your-secret-key

# Retrieval Configuration
RETRIEVAL_TYPE=storage                # Options: storage, mojang, default_skin
RETRIEVAL_CHAIN=storage,mojang,default_skin  # Comma-separated fallback chain

# Caching Configuration
USERNAME_CACHE_SECONDS=28800          # 8 hours (username to UUID cache)
HASH_CACHE_SECONDS=1209600            # 14 days (texture hash cache)
USE_DATABASE_USERNAME_IN_MOJANG_REQUESTS=true

# Admin API (optional)
ADMIN_TOKEN=your-secret-admin-token

# CORS Configuration (optional)
CORS_ALLOWED_ORIGINS=https://example.com,https://app.example.com  # Comma-separated, or * for all
```

## Database Setup

Run the migration file to create the required table:

```bash
psql -U username -d texture_provider -f migrations/001_initial.sql
```

## Building and Running

### Docker

### Using Docker Compose (Recommended)

The easiest way to run the service is with Docker Compose, which includes PostgreSQL:

```bash
# Build and start all services
docker-compose up -d

# View logs
docker-compose logs -f app

# Stop services
docker-compose down
```

The service will be available at `http://localhost:3000`

**Note:** Update the `JWT_PUBLIC_KEY` in `docker-compose.yml` with your actual public key before starting.

### Using Docker Build

Build the image manually:

```bash
docker build -t texture-provider2 .
```

Run the container:

```bash
docker run -d \
  -p 3000:3000 \
  -e DATABASE_URL="postgresql://user:pass@host:5432/texture_provider" \
  -e JWT_PUBLIC_KEY="BASE64_ECDSA_LAUNCHSERVER_KEY" \
  -e STORAGE_TYPE=local \
  texture-provider2
```

### Using Published Docker Images

Images are automatically published to GitHub Container Registry:

```bash
# Pull the latest image
docker pull ghcr.io/your-username/texture-provider2:latest

# Run the image
docker run -d \
  -p 3000:3000 \
  -e DATABASE_URL="postgresql://user:pass@host:5432/texture_provider" \
  -e JWT_PUBLIC_KEY="..." \
  ghcr.io/your-username/texture-provider2:latest
```

### GitHub Actions CI/CD

The project includes a GitHub Actions workflow that:

1. **Builds** Docker images on push to `main`/`master` branches
2. **Publishes** images to GitHub Container Registry (ghcr.io)
3. **Supports** multi-platform builds (linux/amd64, linux/arm64)
4. **Creates** version tags for semantic versioning (v*.*.*)
5. **Generates** SBOM (Software Bill of Materials) for security analysis

**Workflow triggers:**
- Push to main/master branches
- New version tags (e.g., v1.0.0)
- Pull requests (build only, no push)
- Manual workflow dispatch

## Development

Set `DATABASE_URL` environment variable before building:

```bash
export DATABASE_URL="postgresql://username:password@localhost/texture_provider"
cargo run
```

Or use SQLx offline mode (requires `sqlx-cli`):

```bash
cargo install sqlx-cli
cargo sqlx prepare
cargo run --no-default-features
```

### Production

```bash
cargo build --release
./target/release/texture-provider2
```

## API Endpoints

### Public Endpoints

#### GET /get/{uuid}

Get all textures for a user by UUID.

**Response:**
```json
{
  "SKIN": {
    "url": "http://example.com/files/SKIN_HASH",
    "digest": "SHA256_HASH",
    "metadata": {
      "model": "slim"
    }
  },
  "CAPE": {
    "url": "http://example.com/files/CAPE_HASH",
    "digest": "SHA256_HASH"
  }
}
```

#### GET /get/{uuid}/{SKIN|CAPE}

Get a specific texture type for a user.

**Response:**
```json
{
  "url": "http://example.com/files/SKIN_HASH",
  "digest": "SHA256_HASH",
  "metadata": {
    "model": "slim"
  }
}
```

#### GET /download/{SKIN|CAPE}/{uuid}

Download the actual PNG file for a user's texture by UUID.

**Response:** PNG file content

#### GET /download/username/{SKIN|CAPE}/{username}

Download the actual PNG file for a user's texture by username.

**Response:** PNG file content

#### GET /download/{hash}

Download a texture file by its SHA256 hash.

**Response:** PNG file content

#### GET /files/{hash}

Serve a texture file by its SHA256 hash (alternative endpoint).

**Response:** PNG file content

### Authenticated Endpoints

#### POST /upload/{SKIN|CAPE}

Upload a PNG texture file (requires JWT authentication).

**Headers:**
- `Authorization: Bearer JWT_TOKEN`

**Body:** `multipart/form-data`
- `file`: PNG image file
- `options`: JSON string with upload options

**Example:**
```bash
curl -X POST http://localhost:3000/upload/SKIN \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -F "file=@skin.png" \
  -F 'options={"modelSlim":true}'
```

**Response:**
```json
{
  "url": "http://localhost:3000/files/SKIN_HASH",
  "digest": "SHA256_HASH",
  "metadata": {
    "model": "slim"
  }
}
```

### Admin Endpoints

#### POST /api/upload/{SKIN|CAPE}

Upload a texture for a specific user (requires admin token).

**Headers:**
- `Authorization: Bearer ADMIN_TOKEN`

**Body:** `multipart/form-data`
- `file`: PNG image file
- `uuid`: User UUID
- `username`: Username (optional)
- `options`: JSON string with upload options

#### GET /api/get/{username}/{uuid}

Get textures by both username and UUID (requires admin token).

**Headers:**
- `Authorization: Bearer ADMIN_TOKEN`

## Retrieval Strategies

The service supports multiple texture retrieval strategies that can be used individually or chained together:

### Storage Retriever
Retrieves textures from the local database and configured storage backend (local or S3).

### Mojang Retriever
Falls back to the official Mojang API to fetch textures. Supports username-to-UUID resolution with configurable caching.

### Default Skin Retriever
Generates default Steve or Alex skins based on UUID when no texture is found.

### Chain Retriever
Combines multiple strategies with fallback logic. Configure via `RETRIEVAL_CHAIN` environment variable:

```bash
RETRIEVAL_CHAIN=storage,mojang,default_skin
```

This will try storage first, then Mojang API, and finally generate a default skin if all else fails.

## JWT Authentication

The service uses ES256 (ECDSA) JWT tokens compatible with LaunchServer. Include the user UUID in the `uuid` claim:

```json
{
  "uuid": "user-uuid-here",
  "exp": 1234567890
}
```

The JWT public key must be provided in base64 format via the `JWT_PUBLIC_KEY` environment variable.

## Admin Authentication

Admin endpoints require a bearer token specified in the `ADMIN_TOKEN` environment variable:

```bash
curl -H "Authorization: Bearer YOUR_ADMIN_TOKEN" http://localhost:3000/api/upload/SKIN
```

## Storage Types

### Local Storage

Files are stored in the `LOCAL_STORAGE_PATH` directory with SHA256 hash filenames.

### S3 Storage

Files are uploaded to the specified S3 bucket with SHA256 hash keys. Supports any S3-compatible storage (AWS S3, MinIO, etc.).

## Caching

The service implements intelligent caching to reduce external API calls:

- **Username Cache**: Caches username-to-UUID mappings for `USERNAME_CACHE_SECONDS` (default: 8 hours)
- **Hash Cache**: Caches texture hash lookups for `HASH_CACHE_SECONDS` (default: 14 days)
- **Mojang Integration**: Optionally uses database usernames for Mojang API requests via `USE_DATABASE_USERNAME_IN_MOJANG_REQUESTS`

## CORS Configuration

Configure allowed origins via the `CORS_ALLOWED_ORIGINS` environment variable:

- **Specific origins**: `CORS_ALLOWED_ORIGINS=https://example.com,https://app.example.com`
- **All origins** (development only): `CORS_ALLOWED_ORIGINS=*`
- **Not set**: Defaults to allowing all origins (logs a warning)

## Development

### Requirements

- Rust 2021 edition or later
- PostgreSQL 12 or later
- (Optional) AWS S3 account for S3 storage

### Project Structure

```
src/
├── main.rs           # Application entry point and server setup
├── config.rs         # Configuration management and environment variables
├── models.rs         # Data models and database schemas
├── handlers.rs       # HTTP endpoint handlers
├── auth.rs           # JWT authentication and token validation
├── storage/          # Storage backend implementations
│   ├── mod.rs        # Storage trait and factory
│   ├── backend.rs    # Storage backend trait
│   ├── local.rs      # Local filesystem storage
│   └── s3.rs         # S3-compatible storage
└── retrieval/        # Texture retrieval strategies
    ├── mod.rs        # Retrieval trait and factory
    ├── backend.rs    # Retrieval backend trait
    ├── storage_retriever.rs  # Database/storage retrieval
    ├── mojang.rs     # Mojang API integration
    ├── default_skin.rs       # Default skin generation
    └── chain.rs      # Chain retrieval with fallback logic
```

## License

MIT
