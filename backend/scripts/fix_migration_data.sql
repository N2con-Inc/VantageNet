-- ============================================================================
-- Migration Data Cleanup Script
-- ============================================================================
-- Run this script BEFORE upgrading to v0.12.7 if you encounter migration errors:
--   - "insert or update on table interfaces violates foreign key constraint interfaces_subnet_id_fkey"
--   - "duplicate key value violates unique constraint ports_host_id_port_number_protocol_key"
--
-- This script cleans up data issues in the JSONB columns that would cause
-- constraint violations during the normalization migrations.
--
-- Usage:
--   psql -U <user> -d <database> -f fix_migration_data.sql
--   # or via docker:
--   docker exec -i <postgres_container> psql -U <user> -d <database> < fix_migration_data.sql
-- ============================================================================

BEGIN;

-- ============================================================================
-- Step 1: Drop partially-created tables from failed migration attempts
-- ============================================================================
-- If a previous migration attempt failed partway through, these tables may
-- exist in a partial state. Drop them so the migration can start fresh.

DROP TABLE IF EXISTS interfaces CASCADE;
DROP TABLE IF EXISTS ports CASCADE;
DROP TABLE IF EXISTS bindings CASCADE;

-- Also drop columns that may have been added to topologies
ALTER TABLE topologies DROP COLUMN IF EXISTS interfaces;
ALTER TABLE topologies DROP COLUMN IF EXISTS removed_interfaces;
ALTER TABLE topologies DROP COLUMN IF EXISTS ports;
ALTER TABLE topologies DROP COLUMN IF EXISTS removed_ports;
ALTER TABLE topologies DROP COLUMN IF EXISTS bindings;
ALTER TABLE topologies DROP COLUMN IF EXISTS removed_bindings;

-- Remove any migration records for the failed migrations so they'll re-run
DELETE FROM _sqlx_migrations WHERE version IN (
    20251221040000,  -- interfaces_table
    20251221050000,  -- ports_table
    20251221060000,  -- bindings_table
    20251221070000,  -- group_bindings_table
    20251227010000   -- topology_snapshot_migration
);

-- ============================================================================
-- Step 2: Clean up orphaned interface subnet references
-- ============================================================================
-- Interfaces may reference subnet_ids that no longer exist in the subnets table.
-- Remove these interfaces from the JSONB array.

DO $$
DECLARE
    orphaned_count INTEGER;
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'hosts' AND column_name = 'interfaces'
    ) THEN
        SELECT COUNT(*) INTO orphaned_count
        FROM hosts h, jsonb_array_elements(h.interfaces) AS i
        WHERE h.interfaces IS NOT NULL
          AND jsonb_array_length(h.interfaces) > 0
          AND NOT EXISTS (
              SELECT 1 FROM subnets s WHERE s.id = (i->>'subnet_id')::UUID
          );

        IF orphaned_count > 0 THEN
            RAISE NOTICE 'Removing % interface(s) with orphaned subnet references', orphaned_count;

            UPDATE hosts h
            SET interfaces = (
                SELECT COALESCE(jsonb_agg(i), '[]'::jsonb)
                FROM jsonb_array_elements(h.interfaces) AS i
                WHERE EXISTS (
                    SELECT 1 FROM subnets s WHERE s.id = (i->>'subnet_id')::UUID
                )
            )
            WHERE h.interfaces IS NOT NULL
              AND jsonb_array_length(h.interfaces) > 0
              AND EXISTS (
                  SELECT 1 FROM jsonb_array_elements(h.interfaces) AS i
                  WHERE NOT EXISTS (
                      SELECT 1 FROM subnets s WHERE s.id = (i->>'subnet_id')::UUID
                  )
              );
        ELSE
            RAISE NOTICE 'No orphaned interface subnet references found';
        END IF;
    ELSE
        RAISE NOTICE 'hosts.interfaces column does not exist (already migrated)';
    END IF;
END $$;

-- ============================================================================
-- Step 3: Deduplicate ports in hosts.ports JSONB
-- ============================================================================
-- The ports table has UNIQUE(host_id, port_number, protocol).
-- Duplicate ports in JSONB would cause constraint violations.
-- Keep the first occurrence of each (number, protocol) combination.

DO $$
DECLARE
    duplicate_count INTEGER;
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'hosts' AND column_name = 'ports'
    ) THEN
        SELECT COUNT(DISTINCT h.id) INTO duplicate_count
        FROM hosts h
        WHERE h.ports IS NOT NULL
          AND jsonb_array_length(h.ports) > 0
          AND (
              SELECT COUNT(*) FROM jsonb_array_elements(h.ports) AS p
          ) > (
              SELECT COUNT(DISTINCT ((p->>'number')::INTEGER, p->>'protocol'))
              FROM jsonb_array_elements(h.ports) AS p
          );

        IF duplicate_count > 0 THEN
            RAISE NOTICE 'Deduplicating ports on % host(s)', duplicate_count;

            UPDATE hosts h
            SET ports = (
                SELECT jsonb_agg(p ORDER BY ord)
                FROM (
                    SELECT DISTINCT ON ((p->>'number')::INTEGER, p->>'protocol')
                           p, ord
                    FROM jsonb_array_elements(h.ports) WITH ORDINALITY AS arr(p, ord)
                    ORDER BY (p->>'number')::INTEGER, p->>'protocol', ord
                ) deduped
            )
            WHERE h.ports IS NOT NULL
              AND jsonb_array_length(h.ports) > 0
              AND (
                  SELECT COUNT(*) FROM jsonb_array_elements(h.ports) AS p
              ) > (
                  SELECT COUNT(DISTINCT ((p->>'number')::INTEGER, p->>'protocol'))
                  FROM jsonb_array_elements(h.ports) AS p
              );
        ELSE
            RAISE NOTICE 'No duplicate ports found';
        END IF;
    ELSE
        RAISE NOTICE 'hosts.ports column does not exist (already migrated)';
    END IF;
END $$;

COMMIT;

-- ============================================================================
-- Done! You can now restart the server and migrations should succeed.
-- ============================================================================
