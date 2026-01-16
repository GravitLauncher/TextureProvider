# Username-Based Texture Retrieval

This document describes the username-based texture retrieval endpoints that support legacy use cases where the UUID may not be known.

## Overview

The texture provider now supports retrieving textures by username in addition to UUID-based retrieval. This is useful for legacy systems that only have usernames available.

## Important Considerations

⚠️ **Username<->UUID mappings are unreliable**: Usernames can change while UUIDs remain constant. Therefore, username-based lookups should be used with caution and cached for a relatively short time.

### Reliability Assumptions

- **Admin-token requests are reliable**: When an admin token is provided with a username/UUID pair, we assume the mapping is correct and update it in the database.
- **Username-only requests use cached mappings**: These rely on previously stored mappings from admin requests and should have short cache lifetimes (default: 8 hours).

## New Endpoints

### 1. GET /api/get/:username/:uuid

Get all textures for a user by username/UUID pair (admin only).

**Authentication:** Requires admin bearer token

**Purpose:** 
- Returns the same content as `/get/:uuid` 
- Updates the username<->uuid mapping in the database
- Used when you have both username and UUID and want to ensure the mapping is current

**Request:**
```http
GET /api/get/Notch/069a79f4-44e9-4726-a5be-fca90e38aaf5
Authorization: Bearer your-admin-token
```

**Response:**
```json
{
  "SKIN": {
    "url": "http://localhost:3000/files/abc123...",
    "digest": "sha256 hash",
    "metadata": {
      "model": "slim"
    }
  },
  "CAPE": {
    "url": "http://localhost:3000/files/def456...",
    "digest": "sha256 hash",
    "metadata": null
  }
}
```

**Database Operation:**
- Inserts or updates the username_mapping table with the (username, uuid) pair
- Updates the `updated_at` timestamp

### 2. GET /download/username/:texture_type/:username

Download a texture by username.

**Authentication:** None required

**Purpose:**
- Returns the same response as `/download/:texture_type/:uuid` but looks up the UUID from username
- Uses cached username<->uuid mappings
- Falls back to Mojang API to resolve username to UUID if not in local database
- Automatically saves new mappings discovered via Mojang API
- Uses the configured retrieval chain (Storage → Mojang → DefaultSkin) to fetch textures
- Returns cache headers to control caching behavior

**Request:**
```http
GET /download/username/SKIN/Notch
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: image/png
Cache-Control: public, max-age=28800

<binary PNG data>
```

**Cache Headers:**
- `Cache-Control: public, max-age=<USERNAME_CACHE_SECONDS>` (default: 28800 seconds = 8 hours)
- Configurable via the `USERNAME_CACHE_SECONDS` environment variable

**Error Response (username not found):**
```http
HTTP/1.1 404 Not Found
Content-Type: text/plain

Username 'Notch' not found
```

**Error Response (texture not found):**
```http
HTTP/1.1 404 Not Found
Content-Type: text/plain

Texture not found for SKIN
```

## Database Schema

### username_mappings table

```sql
CREATE TABLE username_mappings (
    user_uuid UUID NOT NULL,
    username TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_uuid, username)
);
```

**Indexes:**
- `idx_username_mappings_username` - for fast lookups by username
- `idx_username_mappings_uuid` - for fast lookups by UUID

**Constraints:**
- Unique constraint on (user_uuid, username) ensures each pair is unique

## Configuration

Add to your `.env` file:

```bash
# Username-based Endpoint Cache Configuration
# Cache lifetime in seconds for the /download/username/:texture_type/:username endpoint
# Default is 28800 seconds (8 hours)
# This should be relatively short since username<->uuid mappings are unreliable
USERNAME_CACHE_SECONDS=28800
```

## Usage Examples

### Example 1: Admin updates username mapping

```bash
# Admin has both username and UUID from an external source
curl -X GET \
  'http://localhost:3000/api/get/PlayerName/00000000-0000-0000-0000-000000000000' \
  -H 'Authorization: Bearer your-admin-token'
```

This updates the mapping and returns textures.

### Example 2: Client downloads texture by username

```bash
# Client only knows the username
curl -X GET \
  'http://localhost:3000/download/username/SKIN/PlayerName' \
  --output skin.png
```

This looks up the UUID from the cached username mapping and returns the skin.

## Migration

Run the database migration to create the username_mappings table:

```bash
sqlx migrate run
```

Or with cargo:

```bash
cargo sqlx migrate run
```

## Implementation Details

### Mapping Update Strategy

When `/api/get/:username/:uuid` is called with an admin token:

1. The endpoint performs an `INSERT ... ON CONFLICT DO UPDATE` operation
2. This creates a new mapping or updates an existing one
3. The `updated_at` timestamp is refreshed
4. Multiple username/UUID pairs can coexist for the same user (historical tracking)

### Mapping Lookup Strategy

When `/download/username/:texture_type/:username` is called:

1. The endpoint queries the database for the most recent mapping for the given username
2. If found, it retrieves the texture using the UUID
3. If not found, it returns a 404 error
4. The response includes cache headers to control client-side caching

### Cache Header Rationale

The relatively short cache lifetime (default 8 hours) is chosen because:

- Usernames can change
- Mappings become stale over time
- Shorter cache ensures clients refresh mappings periodically
- Balance between performance and data freshness

## Security Considerations

- The `/api/get/:username/:uuid` endpoint requires an admin token to prevent unauthorized mapping updates
- Admin tokens should be kept secure and rotated regularly
- The `/download/username/:texture_type/:username` endpoint does not require authentication (public access)
- Rate limiting should be considered for public endpoints

## Future Enhancements

Potential improvements for future versions:

1. **Automatic mapping updates**: Use Mojang API to periodically update mappings
2. **Mapping cleanup**: Remove old unused mappings
3. **Username history**: Track username changes over time
4. **Batch mapping operations**: Support updating multiple mappings at once
5. **Mapping validation**: Verify mappings against external sources
