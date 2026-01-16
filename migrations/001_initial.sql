-- Create textures table
CREATE TABLE IF NOT EXISTS textures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uuid UUID NOT NULL,
    texture_type TEXT NOT NULL CHECK (texture_type IN ('SKIN', 'CAPE')),
    file_hash TEXT NOT NULL UNIQUE,
    file_url TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_textures_user_uuid ON textures(user_uuid);
CREATE INDEX IF NOT EXISTS idx_textures_user_type ON textures(user_uuid, texture_type);
CREATE INDEX IF NOT EXISTS idx_textures_hash ON textures(file_hash);

-- Create unique constraint for one texture per type per user
CREATE UNIQUE INDEX IF NOT EXISTS idx_textures_unique_user_type 
    ON textures(user_uuid, texture_type);
