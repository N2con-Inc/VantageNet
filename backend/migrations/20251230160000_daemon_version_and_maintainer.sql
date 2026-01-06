-- Add version and user_id columns to daemons table
-- Version: tracks daemon software version for compatibility checks
-- user_id: tracks which user is responsible for this daemon

-- Add new columns
ALTER TABLE daemons ADD COLUMN version TEXT DEFAULT NULL;
ALTER TABLE daemons ADD COLUMN user_id UUID REFERENCES users(id);

-- Backfill user_id with organization owner
-- Chain: daemon.network_id → networks.organization_id → users(owner)
UPDATE daemons d
SET user_id = (
    SELECT u.id
    FROM users u
    JOIN networks n ON n.organization_id = u.organization_id
    WHERE n.id = d.network_id
      AND u.permissions = 'Owner'
    LIMIT 1
)
WHERE user_id IS NULL;

-- Make user_id NOT NULL after backfill
ALTER TABLE daemons ALTER COLUMN user_id SET NOT NULL;
