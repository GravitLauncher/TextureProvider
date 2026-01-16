-- Create username <-> uuid mapping table
-- This table stores unreliable username mappings that can be updated by admin requests
CREATE TABLE IF NOT EXISTS username_mappings (
    user_uuid UUID NOT NULL,
    username TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_uuid, username)
);

-- Create index for faster lookups by username
CREATE INDEX IF NOT EXISTS idx_username_mappings_username ON username_mappings(username);
-- Create index for faster lookups by uuid
CREATE INDEX IF NOT EXISTS idx_username_mappings_uuid ON username_mappings(user_uuid);
