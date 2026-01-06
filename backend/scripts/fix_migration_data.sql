-- ============================================================================
-- Migration Data Cleanup Script
-- ============================================================================
-- Run this script BEFORE upgrading to v0.12.7 if you encounter migration errors:
--   - "insert or update on table interfaces violates foreign key constraint interfaces_subnet_id_fkey"
--   - "duplicate key value violates unique constraint ports_host_id_port_number_protocol_key"
--   - "duplicate key value violates unique constraint interfaces_pkey"
--
-- This script cleans up data issues in the JSONB columns that would cause
-- constraint violations during the normalization migrations.
--
-- Usage:
--   # Dry run (report only, no changes):
--   psql -U <user> -d <database> -v DRY_RUN=true -f fix_migration_data.sql
--
--   # Actually apply fixes:
--   psql -U <user> -d <database> -f fix_migration_data.sql
--
--   # Via docker:
--   docker exec -i <postgres_container> psql -U <user> -d <database> < fix_migration_data.sql
--   docker exec -i <postgres_container> psql -U <user> -d <database> -v DRY_RUN=true < fix_migration_data.sql
-- ============================================================================

-- Set dry run mode (pass -v DRY_RUN=true to enable)
-- When DRY_RUN is not set, :DRY_RUN expands to literal ':DRY_RUN', so we check for that
\set _dry_run_input :DRY_RUN
SELECT CASE WHEN COALESCE(NULLIF(:'_dry_run_input', ':DRY_RUN'), 'false') = 'true' THEN 'true' ELSE 'false' END AS dry_run \gset

BEGIN;

-- Store dry_run setting for use in PL/pgSQL blocks
SELECT set_config('dry_run.enabled', :'dry_run'::text, true);

-- Show mode
DO $$
BEGIN
    IF current_setting('dry_run.enabled', true) = 'true' THEN
        RAISE NOTICE '';
        RAISE NOTICE '=== DRY RUN MODE === No changes will be made.';
        RAISE NOTICE '';
    ELSE
        RAISE NOTICE '';
        RAISE NOTICE '=== LIVE MODE === Changes will be applied.';
        RAISE NOTICE '';
    END IF;
END $$;

-- ============================================================================
-- Step 1: Drop partially-created tables from failed migration attempts
-- ============================================================================
-- If a previous migration attempt failed partway through, these tables may
-- exist in a partial state. Drop them so the migration can start fresh.

DO $$
BEGIN
    IF current_setting('dry_run.enabled', true) = 'true' THEN
        RAISE NOTICE '[DRY RUN] Would drop tables: bindings, interfaces, ports (if they exist)';
        RAISE NOTICE '[DRY RUN] Would drop topology columns: interfaces, removed_interfaces, ports, removed_ports, bindings, removed_bindings';
        RAISE NOTICE '[DRY RUN] Would delete migration records for versions: 20251221040000, 20251221050000, 20251221060000, 20251221070000, 20251227010000';
    ELSE
        DROP TABLE IF EXISTS bindings CASCADE;
        DROP TABLE IF EXISTS interfaces CASCADE;
        DROP TABLE IF EXISTS ports CASCADE;

        ALTER TABLE topologies DROP COLUMN IF EXISTS interfaces;
        ALTER TABLE topologies DROP COLUMN IF EXISTS removed_interfaces;
        ALTER TABLE topologies DROP COLUMN IF EXISTS ports;
        ALTER TABLE topologies DROP COLUMN IF EXISTS removed_ports;
        ALTER TABLE topologies DROP COLUMN IF EXISTS bindings;
        ALTER TABLE topologies DROP COLUMN IF EXISTS removed_bindings;

        DELETE FROM _sqlx_migrations WHERE version IN (
            20251221040000,
            20251221050000,
            20251221060000,
            20251221070000,
            20251227010000
        );
    END IF;
END $$;

-- ============================================================================
-- Step 2: Build lookup tables for data cleanup
-- ============================================================================
-- We need to track which interfaces will be removed and which ports will be
-- deduplicated, so we can update bindings accordingly.

-- Create empty temp tables first (will be populated if columns exist)
CREATE TEMP TABLE interfaces_to_remove (interface_id UUID PRIMARY KEY);
CREATE TEMP TABLE port_id_mapping (old_port_id UUID PRIMARY KEY, new_port_id UUID);
CREATE TEMP TABLE bindings_to_remove (binding_id UUID PRIMARY KEY);

-- Populate interfaces_to_remove if hosts.interfaces column exists
-- This includes BOTH orphaned subnet refs AND duplicate interfaces
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'hosts' AND column_name = 'interfaces'
    ) THEN
        -- First: interfaces with orphaned subnet references
        INSERT INTO interfaces_to_remove (interface_id)
        SELECT (i->>'id')::UUID
        FROM hosts h, jsonb_array_elements(h.interfaces) AS i
        WHERE h.interfaces IS NOT NULL
          AND jsonb_array_length(h.interfaces) > 0
          AND NOT EXISTS (
              SELECT 1 FROM subnets s WHERE s.id = (i->>'subnet_id')::UUID
          );

        -- Second: duplicate interfaces (keep first by position, remove rest)
        -- UNIQUE constraint is on (host_id, subnet_id, ip_address)
        INSERT INTO interfaces_to_remove (interface_id)
        SELECT interface_id FROM (
            SELECT
                (i->>'id')::UUID AS interface_id,
                ROW_NUMBER() OVER (
                    PARTITION BY h.id, i->>'subnet_id', i->>'ip_address'
                    ORDER BY ord
                ) AS rn
            FROM hosts h, jsonb_array_elements(h.interfaces) WITH ORDINALITY AS arr(i, ord)
            WHERE h.interfaces IS NOT NULL
              AND jsonb_array_length(h.interfaces) > 0
              -- Only consider interfaces with valid subnets (orphaned ones already added above)
              AND EXISTS (
                  SELECT 1 FROM subnets s WHERE s.id = (i->>'subnet_id')::UUID
              )
        ) ranked
        WHERE rn > 1
        ON CONFLICT DO NOTHING;
    END IF;
END $$;

-- ============================================================================
-- Step 2b: Fix globally duplicate interface UUIDs
-- ============================================================================
-- The interfaces table has PRIMARY KEY on id.
-- If the same interface UUID appears on different hosts, the migration will fail.
-- This can happen if hosts were duplicated. We regenerate UUIDs for duplicates,
-- keeping the first occurrence unchanged.

CREATE TEMP TABLE global_interface_duplicates (
    host_id UUID,
    old_interface_id UUID,
    new_interface_id UUID
);

DO $$
DECLARE
    dup_count INTEGER;
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'hosts' AND column_name = 'interfaces'
    ) THEN
        -- Find interface UUIDs that appear on multiple hosts and mark all but first for UUID regeneration
        INSERT INTO global_interface_duplicates (host_id, old_interface_id, new_interface_id)
        SELECT host_id, interface_id, gen_random_uuid()
        FROM (
            SELECT
                h.id AS host_id,
                (i->>'id')::UUID AS interface_id,
                ROW_NUMBER() OVER (PARTITION BY (i->>'id')::UUID ORDER BY h.id) AS rn
            FROM hosts h, jsonb_array_elements(h.interfaces) AS i
            WHERE h.interfaces IS NOT NULL
              AND jsonb_array_length(h.interfaces) > 0
        ) ranked
        WHERE rn > 1;

        SELECT COUNT(*) INTO dup_count FROM global_interface_duplicates;

        IF dup_count > 0 THEN
            IF current_setting('dry_run.enabled', true) = 'true' THEN
                RAISE NOTICE '[DRY RUN] Would regenerate UUIDs for % interface(s) with globally duplicate IDs', dup_count;
            ELSE
                RAISE NOTICE 'Regenerating UUIDs for % interface(s) with globally duplicate IDs', dup_count;

                UPDATE hosts h
                SET interfaces = (
                    SELECT jsonb_agg(
                        CASE
                            WHEN (i->>'id')::UUID IN (
                                SELECT old_interface_id FROM global_interface_duplicates
                                WHERE global_interface_duplicates.host_id = h.id
                            )
                            THEN jsonb_set(
                                i,
                                '{id}',
                                to_jsonb((
                                    SELECT new_interface_id::TEXT
                                    FROM global_interface_duplicates
                                    WHERE global_interface_duplicates.host_id = h.id
                                      AND global_interface_duplicates.old_interface_id = (i->>'id')::UUID
                                ))
                            )
                            ELSE i
                        END
                    )
                    FROM jsonb_array_elements(h.interfaces) AS i
                )
                WHERE h.id IN (SELECT host_id FROM global_interface_duplicates);
            END IF;
        ELSE
            RAISE NOTICE 'No globally duplicate interface UUIDs found';
        END IF;
    END IF;
END $$;

-- Populate port_id_mapping if hosts.ports column exists
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'hosts' AND column_name = 'ports'
    ) THEN
        INSERT INTO port_id_mapping (old_port_id, new_port_id)
        WITH port_data AS (
            SELECT
                h.id AS host_id,
                (p->>'id')::UUID AS port_id,
                (p->>'number')::INTEGER AS port_number,
                p->>'protocol' AS protocol,
                ROW_NUMBER() OVER (
                    PARTITION BY h.id, (p->>'number')::INTEGER, p->>'protocol'
                    ORDER BY ord
                ) AS rn
            FROM hosts h, jsonb_array_elements(h.ports) WITH ORDINALITY AS arr(p, ord)
            WHERE h.ports IS NOT NULL AND jsonb_array_length(h.ports) > 0
        ),
        first_ports AS (
            SELECT host_id, port_number, protocol, port_id AS kept_port_id
            FROM port_data WHERE rn = 1
        )
        SELECT pd.port_id AS old_port_id, fp.kept_port_id AS new_port_id
        FROM port_data pd
        JOIN first_ports fp ON pd.host_id = fp.host_id
            AND pd.port_number = fp.port_number
            AND pd.protocol = fp.protocol
        WHERE pd.rn > 1;
    END IF;
END $$;

-- Populate bindings_to_remove if services.bindings column exists
-- Must be done BEFORE Step 3 modifies services.bindings
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'services' AND column_name = 'bindings'
    ) THEN
        INSERT INTO bindings_to_remove (binding_id)
        SELECT (b->>'id')::UUID
        FROM services s, jsonb_array_elements(s.bindings) AS b
        WHERE s.bindings IS NOT NULL
          AND jsonb_array_length(s.bindings) > 0
          AND b->>'type' = 'Interface'
          AND (b->>'interface_id')::UUID IN (SELECT interface_id FROM interfaces_to_remove);
    END IF;
END $$;

-- ============================================================================
-- Step 3: Clean up bindings in services.bindings JSONB
-- ============================================================================
-- Before removing interfaces or deduplicating ports, update bindings to:
-- 1. Remove bindings that reference interfaces being removed
-- 2. Remap port_id references for deduplicated ports

DO $$
DECLARE
    binding_cleanup_count INTEGER := 0;
    binding_remap_count INTEGER := 0;
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'services' AND column_name = 'bindings'
    ) THEN
        -- Count bindings referencing removed interfaces
        SELECT COUNT(*) INTO binding_cleanup_count
        FROM services s, jsonb_array_elements(s.bindings) AS b
        WHERE s.bindings IS NOT NULL
          AND jsonb_array_length(s.bindings) > 0
          AND (b->>'interface_id')::UUID IN (SELECT interface_id FROM interfaces_to_remove);

        -- Count bindings referencing deduplicated ports
        SELECT COUNT(*) INTO binding_remap_count
        FROM services s, jsonb_array_elements(s.bindings) AS b
        WHERE s.bindings IS NOT NULL
          AND jsonb_array_length(s.bindings) > 0
          AND (b->>'port_id')::UUID IN (SELECT old_port_id FROM port_id_mapping);

        IF binding_cleanup_count > 0 OR binding_remap_count > 0 THEN
            IF current_setting('dry_run.enabled', true) = 'true' THEN
                RAISE NOTICE '[DRY RUN] Would clean up bindings: removing % with orphaned interfaces, remapping % with deduplicated ports',
                    binding_cleanup_count, binding_remap_count;
            ELSE
                RAISE NOTICE 'Cleaning up bindings: removing % with orphaned interfaces, remapping % with deduplicated ports',
                    binding_cleanup_count, binding_remap_count;

                -- Update bindings: remove orphaned interface refs, remap deduplicated port refs
                UPDATE services s
            SET bindings = (
                SELECT COALESCE(jsonb_agg(
                    CASE
                        -- Remap port_id for deduplicated ports
                        WHEN b->>'type' = 'Port' AND (b->>'port_id')::UUID IN (SELECT old_port_id FROM port_id_mapping)
                        THEN jsonb_set(
                            -- Also remap interface_id if it's being removed (set to null for "all interfaces")
                            CASE
                                WHEN (b->>'interface_id')::UUID IN (SELECT interface_id FROM interfaces_to_remove)
                                THEN b - 'interface_id'
                                ELSE b
                            END,
                            '{port_id}',
                            to_jsonb((SELECT new_port_id FROM port_id_mapping WHERE old_port_id = (b->>'port_id')::UUID)::TEXT)
                        )
                        -- Remove interface_id from port bindings if interface is being removed
                        WHEN b->>'type' = 'Port' AND (b->>'interface_id')::UUID IN (SELECT interface_id FROM interfaces_to_remove)
                        THEN b - 'interface_id'
                        ELSE b
                    END
                ), '[]'::jsonb)
                FROM jsonb_array_elements(s.bindings) AS b
                -- Filter out Interface bindings that reference removed interfaces
                WHERE NOT (
                    b->>'type' = 'Interface'
                    AND (b->>'interface_id')::UUID IN (SELECT interface_id FROM interfaces_to_remove)
                )
            )
            WHERE s.bindings IS NOT NULL
              AND jsonb_array_length(s.bindings) > 0
              AND (
                  -- Has binding referencing removed interface
                  EXISTS (
                      SELECT 1 FROM jsonb_array_elements(s.bindings) AS b
                      WHERE (b->>'interface_id')::UUID IN (SELECT interface_id FROM interfaces_to_remove)
                  )
                  OR
                  -- Has binding referencing deduplicated port
                  EXISTS (
                      SELECT 1 FROM jsonb_array_elements(s.bindings) AS b
                      WHERE (b->>'port_id')::UUID IN (SELECT old_port_id FROM port_id_mapping)
                  )
              );
            END IF;
        ELSE
            RAISE NOTICE 'No binding cleanup needed';
        END IF;
    ELSE
        RAISE NOTICE 'services.bindings column does not exist (already migrated)';
    END IF;
END $$;

-- ============================================================================
-- Step 4: Remove problematic interfaces (orphaned subnet refs + duplicates)
-- ============================================================================
-- The interfaces_to_remove table contains:
-- 1. Interfaces with subnet_ids that don't exist in subnets table
-- 2. Duplicate interfaces (same host_id, subnet_id, ip_address) - keeping first only
-- Remove all of these from the hosts.interfaces JSONB array.

DO $$
DECLARE
    remove_count INTEGER;
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'hosts' AND column_name = 'interfaces'
    ) THEN
        SELECT COUNT(*) INTO remove_count FROM interfaces_to_remove;

        IF remove_count > 0 THEN
            IF current_setting('dry_run.enabled', true) = 'true' THEN
                RAISE NOTICE '[DRY RUN] Would remove % interface(s) (orphaned subnet refs + duplicates)', remove_count;
            ELSE
                RAISE NOTICE 'Removing % interface(s) (orphaned subnet refs + duplicates)', remove_count;

                UPDATE hosts h
                SET interfaces = (
                    SELECT COALESCE(jsonb_agg(i), '[]'::jsonb)
                    FROM jsonb_array_elements(h.interfaces) AS i
                    WHERE (i->>'id')::UUID NOT IN (SELECT interface_id FROM interfaces_to_remove)
                )
                WHERE h.interfaces IS NOT NULL
                  AND jsonb_array_length(h.interfaces) > 0
                  AND EXISTS (
                      SELECT 1 FROM jsonb_array_elements(h.interfaces) AS i
                      WHERE (i->>'id')::UUID IN (SELECT interface_id FROM interfaces_to_remove)
                  );
            END IF;
        ELSE
            RAISE NOTICE 'No problematic interfaces found';
        END IF;
    ELSE
        RAISE NOTICE 'hosts.interfaces column does not exist (already migrated)';
    END IF;
END $$;

-- ============================================================================
-- Step 5: Deduplicate ports in hosts.ports JSONB
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
        SELECT COUNT(DISTINCT host_id) INTO duplicate_count
        FROM (
            SELECT h.id AS host_id
            FROM hosts h
            WHERE h.ports IS NOT NULL
              AND jsonb_array_length(h.ports) > 0
              AND (
                  SELECT COUNT(*) FROM jsonb_array_elements(h.ports) AS p
              ) > (
                  SELECT COUNT(DISTINCT ((p->>'number')::INTEGER, p->>'protocol'))
                  FROM jsonb_array_elements(h.ports) AS p
              )
        ) dupes;

        IF duplicate_count > 0 THEN
            IF current_setting('dry_run.enabled', true) = 'true' THEN
                RAISE NOTICE '[DRY RUN] Would deduplicate ports on % host(s)', duplicate_count;
            ELSE
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
            END IF;
        ELSE
            RAISE NOTICE 'No duplicate ports found';
        END IF;
    ELSE
        RAISE NOTICE 'hosts.ports column does not exist (already migrated)';
    END IF;
END $$;

-- ============================================================================
-- Step 6: Deduplicate user network_ids arrays
-- ============================================================================
-- The user_network_access table has UNIQUE(user_id, network_id).
-- If users.network_ids contains duplicates, the migration will fail.

DO $$
DECLARE
    duplicate_count INTEGER;
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'users' AND column_name = 'network_ids'
    ) THEN
        SELECT COUNT(*) INTO duplicate_count
        FROM users u
        WHERE u.network_ids IS NOT NULL
          AND array_length(u.network_ids, 1) > 0
          AND array_length(u.network_ids, 1) > (
              SELECT COUNT(DISTINCT nid) FROM unnest(u.network_ids) AS nid
          );

        IF duplicate_count > 0 THEN
            IF current_setting('dry_run.enabled', true) = 'true' THEN
                RAISE NOTICE '[DRY RUN] Would deduplicate network_ids on % user(s)', duplicate_count;
            ELSE
                RAISE NOTICE 'Deduplicating network_ids on % user(s)', duplicate_count;

                UPDATE users u
                SET network_ids = (
                    SELECT ARRAY(SELECT DISTINCT unnest(u.network_ids))
                )
                WHERE u.network_ids IS NOT NULL
                  AND array_length(u.network_ids, 1) > 0
                  AND array_length(u.network_ids, 1) > (
                      SELECT COUNT(DISTINCT nid) FROM unnest(u.network_ids) AS nid
                  );
            END IF;
        ELSE
            RAISE NOTICE 'No duplicate network_ids found';
        END IF;
    ELSE
        RAISE NOTICE 'users.network_ids column does not exist (already migrated)';
    END IF;
END $$;

-- ============================================================================
-- Step 7: Clean up group service_bindings that reference removed bindings
-- ============================================================================
-- The group_bindings table references bindings(id).
-- If we removed interface bindings (due to orphaned subnet), we need to also
-- remove those binding IDs from groups.group_type->'service_bindings'.

DO $$
DECLARE
    cleanup_count INTEGER;
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'groups' AND column_name = 'group_type'
          AND data_type = 'jsonb'
    ) THEN
        -- bindings_to_remove was populated in Step 2 (before services.bindings was modified)
        SELECT COUNT(*) INTO cleanup_count
        FROM groups g, jsonb_array_elements_text(g.group_type->'service_bindings') AS binding_id
        WHERE g.group_type->'service_bindings' IS NOT NULL
          AND jsonb_array_length(g.group_type->'service_bindings') > 0
          AND binding_id::UUID IN (SELECT binding_id FROM bindings_to_remove);

        IF cleanup_count > 0 THEN
            IF current_setting('dry_run.enabled', true) = 'true' THEN
                RAISE NOTICE '[DRY RUN] Would remove % binding reference(s) from group service_bindings', cleanup_count;
            ELSE
                RAISE NOTICE 'Removing % binding reference(s) from group service_bindings', cleanup_count;

                UPDATE groups g
                SET group_type = jsonb_set(
                    g.group_type,
                    '{service_bindings}',
                    (
                        SELECT COALESCE(jsonb_agg(binding_id), '[]'::jsonb)
                        FROM jsonb_array_elements_text(g.group_type->'service_bindings') AS binding_id
                        WHERE binding_id::UUID NOT IN (SELECT binding_id FROM bindings_to_remove)
                    )
                )
                WHERE g.group_type->'service_bindings' IS NOT NULL
                  AND jsonb_array_length(g.group_type->'service_bindings') > 0
                  AND EXISTS (
                      SELECT 1 FROM jsonb_array_elements_text(g.group_type->'service_bindings') AS bid
                      WHERE bid::UUID IN (SELECT binding_id FROM bindings_to_remove)
                  );
            END IF;
        ELSE
            RAISE NOTICE 'No group service_binding cleanup needed';
        END IF;
    ELSE
        RAISE NOTICE 'groups.group_type JSONB column does not exist (already migrated)';
    END IF;
END $$;

-- Clean up temp tables
DROP TABLE IF EXISTS interfaces_to_remove;
DROP TABLE IF EXISTS port_id_mapping;
DROP TABLE IF EXISTS bindings_to_remove;
DROP TABLE IF EXISTS global_interface_duplicates;

-- Commit or rollback based on dry-run mode
DO $$
BEGIN
    IF current_setting('dry_run.enabled', true) = 'true' THEN
        RAISE NOTICE '';
        RAISE NOTICE '=== DRY RUN COMPLETE ===';
        RAISE NOTICE 'No changes were made. Review the output above.';
        RAISE NOTICE 'To apply changes, run without -v DRY_RUN=true';
    ELSE
        RAISE NOTICE '';
        RAISE NOTICE '=== CLEANUP COMPLETE ===';
        RAISE NOTICE 'You can now restart the server and migrations should succeed.';
    END IF;
END $$;

-- Use \gexec to conditionally execute COMMIT or ROLLBACK
SELECT CASE WHEN :'dry_run' = 'true' THEN 'ROLLBACK' ELSE 'COMMIT' END \gexec
