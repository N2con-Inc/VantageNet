-- User API Keys table
-- Allows users to create API keys for programmatic access with configurable permissions
CREATE TABLE user_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key TEXT NOT NULL UNIQUE,                                        -- SHA-256 hash of the key
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,    -- Owner of the key
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    permissions TEXT NOT NULL DEFAULT 'Viewer',                      -- UserOrgPermissions: Owner/Admin/Member/Viewer
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used TIMESTAMPTZ,                                           -- Tracks when key was last used for auth
    expires_at TIMESTAMPTZ,                                          -- Optional expiration date
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,                        -- Enable/disable without deletion
    tags UUID[] NOT NULL DEFAULT '{}'                                -- Tag associations
);

-- Junction table for explicit network access grants
-- User API keys require explicit network access (no auto-access like Owner/Admin users)
CREATE TABLE user_api_key_network_access (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_key_id UUID NOT NULL REFERENCES user_api_keys(id) ON DELETE CASCADE,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(api_key_id, network_id)
);

-- Indexes for efficient lookups
CREATE INDEX idx_user_api_keys_key ON user_api_keys(key);
CREATE INDEX idx_user_api_keys_user ON user_api_keys(user_id);
CREATE INDEX idx_user_api_keys_org ON user_api_keys(organization_id);
CREATE INDEX idx_user_api_key_network_access_key ON user_api_key_network_access(api_key_id);
CREATE INDEX idx_user_api_key_network_access_network ON user_api_key_network_access(network_id);
