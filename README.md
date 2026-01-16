# Texture Provider Service

A RESTful web service for uploading and managing PNG texture files (SKIN and CAPE) with support for local or S3 storage.

## Features

- Upload PNG files with SHA256 hash-based naming
- JWT-based authentication using ES256 algorithm
- Support for both local and S3 storage
- PostgreSQL database for metadata storage
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
STORAGE_TYPE=local
LOCAL_STORAGE_PATH=./uploads
SERVER_PORT=3000

# S3 Storage (required if STORAGE_TYPE=s3)
S3_BUCKET=your-bucket-name
S3_REGION=us-east-1
S3_ENDPOINT=https://s3.amazonaws.com
S3_ACCESS_KEY=your-access-key
S3_SECRET_KEY=your-secret-key
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

### GET /get/{uuid}

Get all textures for a user.

**Response:**
```json
{
  "SKIN": {
    "url": "http://example.com/SKIN_HASH.png",
    "digest": "SHA256_HASH",
    "metadata": {
      "model": "slim"
    }
  },
  "CAPE": {
    "url": "http://example.com/CAPE_HASH.png",
    "digest": "SHA256_HASH"
  }
}
```

### GET /get/{uuid}/{SKIN|CAPE}

Get a specific texture type for a user.

**Response:**
```json
{
  "url": "http://example.com/SKIN_HASH.png",
  "digest": "SHA256_HASH",
  "metadata": {
    "model": "slim"
  }
}
```

### POST /upload

Upload a PNG texture file.

**Headers:**
- `Authorization: Bearer JWT_TOKEN`

**Body:** `multipart/form-data`
- `file`: PNG image file
- `options`: JSON string with upload options

**Example:**
```bash
curl -X POST http://localhost:3000/upload \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -F "file=@skin.png" \
  -F 'options={"modelSlim":true}'
```

**Response:**
```json
{
  "url": "http://localhost:3000/SKIN_HASH.png",
  "digest": "SHA256_HASH",
  "metadata": {
    "model": "slim"
  }
}
```

### GET /download/{SKIN|CAPE}/{uuid}

Download the actual PNG file for a user's texture.

**Response:** PNG file content

## JWT Authentication

The service uses ES256 JWT tokens. Include the user UUID in the `uuid` claim:

```json
{
  "uuid": "user-uuid-here",
  "exp": 1234567890
}
```

The JWT public key must be provided in the `JWT_PUBLIC_KEY` environment variable.

## Storage Types

### Local Storage

Files are stored in the `LOCAL_STORAGE_PATH` directory with SHA256 hash filenames.

### S3 Storage

Files are uploaded to the specified S3 bucket with SHA256 hash keys.

## Development

### Requirements

- Rust 2021 edition or later
- PostgreSQL 12 or later
- (Optional) AWS S3 account for S3 storage

### Project Structure

```
src/
├── main.rs       # Application entry point
├── config.rs     # Configuration management
├── models.rs     # Data models
├── handlers.rs   # HTTP endpoint handlers
├── auth.rs       # JWT authentication
└── storage.rs    # File storage abstraction
```

## License

MIT
