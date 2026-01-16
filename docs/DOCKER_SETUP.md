# Docker Setup Summary

This document provides an overview of the Docker configuration for the Texture Provider Service.

## Files Created

### 1. Dockerfile
- **Multi-stage build** using Rust Alpine builder
- **Runtime stage** using minimal Alpine image
- **Non-root user** for security
- **Optimized** for production use
- **Supports** both local and S3 storage

### 2. docker-compose.yml
- **PostgreSQL service** with health checks
- **Application service** with proper dependencies
- **Volume management** for data persistence
- **Environment configuration** for local development
- **Port mapping** (3000:3000)

### 3. .github/workflows/docker-publish.yml
- **Automated builds** on push to main/master
- **Multi-platform support** (amd64, arm64)
- **Semantic versioning** support
- **GitHub Container Registry** publishing
- **SBOM generation** for security

### 4. .dockerignore
- **Excludes** unnecessary files from build context
- **Reduces** build time and image size
- **Improves** build performance

## Quick Start

### Using Docker Compose
```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

### Manual Docker Build
```bash
# Build image
docker build -t texture-provider2 .

# Run container
docker run -p 3000:3000 texture-provider2
```

## GitHub Actions CI/CD

The workflow automatically:
1. **Triggers** on push to main/master, version tags, and PRs
2. **Builds** multi-platform Docker images
3. **Pushes** to GitHub Container Registry (ghcr.io)
4. **Generates** SBOM for security analysis

### Required GitHub Settings
- Enable GitHub Packages in repository settings
- Ensure proper permissions for workflow

### Image Tags
- `latest` - Latest build from main branch
- `v1.0.0` - Semantic version tags
- `main-abc1234` - Branch name with commit SHA
- `pr-42` - Pull request number

## Environment Configuration

The Docker setup uses the same environment variables as the native application:

### Required Variables
- `DATABASE_URL` - PostgreSQL connection string
- `JWT_PUBLIC_KEY` - ES256 public key in PEM format

### Optional Variables
- `BASE_URL` - Base URL for texture URLs (default: http://localhost:3000)
- `STORAGE_TYPE` - local or s3 (default: local)
- `SERVER_PORT` - Server port (default: 3000)

### S3 Configuration (if using S3)
- `S3_BUCKET` - Bucket name
- `S3_REGION` - AWS region
- `S3_ENDPOINT` - S3 endpoint URL
- `S3_ACCESS_KEY` - Access key
- `S3_SECRET_KEY` - Secret key

## Production Considerations

1. **Security**
   - Use secrets management for sensitive data
   - Keep images updated with security patches
   - Run containers as non-root user

2. **Performance**
   - Use multi-stage builds for smaller images
   - Enable BuildKit for faster builds
   - Use layer caching effectively

3. **Monitoring**
   - Implement health checks
   - Set up logging aggregation
   - Monitor resource usage

4. **Backup**
   - Regular PostgreSQL backups
   - Volume snapshots for uploaded files
   - Disaster recovery plan

## Troubleshooting

### Build Issues
- Check Rust version compatibility
- Verify all dependencies are available
- Ensure sufficient disk space

### Runtime Issues
- Verify database connectivity
- Check environment variables
- Review container logs

### Image Publishing
- Verify GitHub token permissions
- Check repository settings
- Review workflow logs
