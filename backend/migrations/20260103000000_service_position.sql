-- Add position column to services table for ordering services within a host
ALTER TABLE services ADD COLUMN position INTEGER NOT NULL DEFAULT 0;

-- Create index for efficient ordering by position
CREATE INDEX IF NOT EXISTS idx_services_host_position ON services(host_id, position);
