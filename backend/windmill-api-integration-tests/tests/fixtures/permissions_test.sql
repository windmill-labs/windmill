-- Permissions test fixture
-- This sets up a comprehensive permission testing scenario

-- ============================================
-- USERS
-- ============================================

-- Admin user (may already exist from base.sql, use ON CONFLICT)
INSERT INTO usr (workspace_id, email, username, is_admin, role)
VALUES ('test-workspace', 'admin@windmill.dev', 'admin', true, 'Admin')
ON CONFLICT (workspace_id, username) DO NOTHING;

-- Regular users (non-admin)
INSERT INTO usr (workspace_id, email, username, is_admin, role)
VALUES
    ('test-workspace', 'alice@windmill.dev', 'alice', false, 'Developer'),
    ('test-workspace', 'bob@windmill.dev', 'bob', false, 'Developer'),
    ('test-workspace', 'charlie@windmill.dev', 'charlie', false, 'Developer');

-- Operator user (can execute but cannot create/update scripts, flows, apps)
INSERT INTO usr (workspace_id, email, username, is_admin, operator, role)
VALUES
    ('test-workspace', 'operator@windmill.dev', 'operator', false, true, 'Operator');

-- Add users to password table (use ON CONFLICT since admin may exist from base.sql)
INSERT INTO password (email, password_hash, login_type, super_admin, verified, name)
VALUES
    ('admin@windmill.dev', 'dummy_hash', 'password', false, true, 'Admin User'),
    ('alice@windmill.dev', 'dummy_hash', 'password', false, true, 'Alice'),
    ('bob@windmill.dev', 'dummy_hash', 'password', false, true, 'Bob'),
    ('charlie@windmill.dev', 'dummy_hash', 'password', false, true, 'Charlie'),
    ('operator@windmill.dev', 'dummy_hash', 'password', false, true, 'Operator')
ON CONFLICT (email) DO NOTHING;

-- ============================================
-- TOKENS for authentication
-- ============================================

-- Tokens associated with emails (workspace-scoped)
-- The auth system will look up the user by email in the usr table
-- Note: tokens must be at least 10 characters (TOKEN_PREFIX_LEN)
INSERT INTO token (token, email, label, super_admin, owner, workspace_id)
VALUES
    ('ADMIN_TOKEN_TEST', 'admin@windmill.dev', 'Admin token', false, 'u/admin', 'test-workspace'),
    ('ALICE_TOKEN_TEST', 'alice@windmill.dev', 'Alice token', false, 'u/alice', 'test-workspace'),
    ('BOB_TOKEN_TEST12', 'bob@windmill.dev', 'Bob token', false, 'u/bob', 'test-workspace'),
    ('CHARLIE_TOKEN_01', 'charlie@windmill.dev', 'Charlie token', false, 'u/charlie', 'test-workspace'),
    ('OPERATOR_TOKEN_1', 'operator@windmill.dev', 'Operator token', false, 'u/operator', 'test-workspace');

-- ============================================
-- GROUPS
-- ============================================

-- 'developers' group - Charlie is a member
INSERT INTO group_ (workspace_id, name, summary, extra_perms)
VALUES ('test-workspace', 'developers', 'Developer group', '{}');

-- 'editors' group - Bob is a member (has write access to some folders)
INSERT INTO group_ (workspace_id, name, summary, extra_perms)
VALUES ('test-workspace', 'editors', 'Editor group', '{}');

-- Group memberships
INSERT INTO usr_to_group (workspace_id, group_, usr)
VALUES
    ('test-workspace', 'developers', 'charlie'),
    ('test-workspace', 'editors', 'bob');

-- ============================================
-- FOLDERS
-- ============================================

-- 'shared' folder:
--   - Alice has read-only access (extra_perms: u/alice -> false)
--   - Bob has write access (extra_perms: u/bob -> true)
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'shared', 'Shared Folder', '{"u/admin"}',
        '{"u/alice": false, "u/bob": true}', 'admin');

-- 'team' folder:
--   - 'developers' group has read-only access
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'team', 'Team Folder', '{"u/admin"}',
        '{"g/developers": false}', 'admin');

-- 'editable' folder:
--   - 'editors' group has write access
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'editable', 'Editable Folder', '{"u/admin"}',
        '{"g/editors": true}', 'admin');

-- 'admin_only' folder:
--   - Only admin has access (but we'll add item-level perms for specific items)
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'admin_only', 'Admin Only Folder', '{"u/admin"}',
        '{}', 'admin');

-- 'alice_owned' folder:
--   - Alice is an owner (owners must also be in extra_perms for get_folders_for_user to find them)
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'alice_owned', 'Alice Owned Folder', '{"u/alice"}',
        '{"u/alice": true}', 'alice');

-- ============================================
-- SCRIPTS
-- ============================================

-- Alice's personal script
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 1001, 'u/alice/my_script',
        'export function main() { return "alice script"; }',
        'deno', 'script', 'alice', '{}', 'Alice script', '', '', '{}');

-- Bob's personal script
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 1002, 'u/bob/my_script',
        'export function main() { return "bob script"; }',
        'deno', 'script', 'bob', '{}', 'Bob script', '', '', '{}');

-- Script in shared folder (accessible by Alice read-only, Bob write)
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 1003, 'f/shared/public_script',
        'export function main() { return "public"; }',
        'deno', 'script', 'admin', '{}', 'Public script', '', '', '{}');

-- Script in team folder (accessible by developers group)
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 1004, 'f/team/team_script',
        'export function main() { return "team"; }',
        'deno', 'script', 'admin', '{}', 'Team script', '', '', '{}');

-- Script in admin_only folder but with explicit share to Alice
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 1005, 'f/admin_only/shared_with_alice',
        'export function main() { return "shared"; }',
        'deno', 'script', 'admin', '{}', 'Shared with Alice', '', '',
        '{"u/alice": false}');

-- Script in alice_owned folder
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 1006, 'f/alice_owned/owner_script',
        'export function main() { return "owner"; }',
        'deno', 'script', 'alice', '{}', 'Owner script', '', '', '{}');

-- Alice's script with extra_perms sharing to Bob (read-only)
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 1007, 'u/alice/extra_shared_script',
        'export function main() { return "extra shared"; }',
        'deno', 'script', 'alice', '{}', 'Extra shared script', '', '',
        '{"u/bob": false}');

-- ============================================
-- FLOWS (with flow_versions)
-- ============================================

-- Create flows first (without versions)
INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, schema, extra_perms)
VALUES
    ('test-workspace', 'u/alice/my_flow', 'Alice flow', '', '{"modules": []}', 'alice', NOW(), '{}', '{}'),
    ('test-workspace', 'u/bob/my_flow', 'Bob flow', '', '{"modules": []}', 'bob', NOW(), '{}', '{}'),
    ('test-workspace', 'f/shared/shared_flow', 'Shared flow', '', '{"modules": []}', 'admin', NOW(), '{}', '{}');

-- Create flow versions
INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by, created_at)
VALUES
    (1001, 'test-workspace', 'u/alice/my_flow', '{"modules": []}', '{}', 'alice', NOW()),
    (1002, 'test-workspace', 'u/bob/my_flow', '{"modules": []}', '{}', 'bob', NOW()),
    (1003, 'test-workspace', 'f/shared/shared_flow', '{"modules": []}', '{}', 'admin', NOW());

-- Update flows with version references
UPDATE flow SET versions = ARRAY[1001::bigint] WHERE path = 'u/alice/my_flow' AND workspace_id = 'test-workspace';
UPDATE flow SET versions = ARRAY[1002::bigint] WHERE path = 'u/bob/my_flow' AND workspace_id = 'test-workspace';
UPDATE flow SET versions = ARRAY[1003::bigint] WHERE path = 'f/shared/shared_flow' AND workspace_id = 'test-workspace';

-- ============================================
-- RESOURCES
-- ============================================

-- Alice's personal resource
INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/alice/my_resource', '{"key": "alice_value"}',
        'Alice resource', 'object', '{}', 'alice');

-- Bob's personal resource
INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/bob/my_resource', '{"key": "bob_value"}',
        'Bob resource', 'object', '{}', 'bob');

-- ============================================
-- VARIABLES
-- ============================================

-- Alice's personal variable (non-secret for testing permissions, not encryption)
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/alice/my_variable', 'alice_value', false,
        'Alice variable', '{}');

-- Bob's personal variable (non-secret for testing permissions, not encryption)
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/bob/my_variable', 'bob_value', false,
        'Bob variable', '{}');

-- ============================================
-- SCHEDULES
-- ============================================

-- Alice's personal schedule
INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, enabled, script_path, args, is_flow, email, timezone, extra_perms)
VALUES ('test-workspace', 'u/alice/my_schedule', 'alice', NOW(), '0 * * * *', false,
        'u/alice/my_script', '{}', false, 'alice@windmill.dev', 'UTC', '{}');

-- Bob's personal schedule
INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, enabled, script_path, args, is_flow, email, timezone, extra_perms)
VALUES ('test-workspace', 'u/bob/my_schedule', 'bob', NOW(), '0 * * * *', false,
        'u/bob/my_script', '{}', false, 'bob@windmill.dev', 'UTC', '{}');

-- ============================================
-- APPS (with app_versions)
-- ============================================

-- Alice's personal app
INSERT INTO app (id, workspace_id, path, summary, versions, policy, extra_perms)
VALUES (2001, 'test-workspace', 'u/alice/my_app', 'Alice app', '{}',
        '{"on_behalf_of": "u/alice", "on_behalf_of_email": "alice@windmill.dev", "execution_mode": "viewer"}', '{}');

-- Shared folder app
INSERT INTO app (id, workspace_id, path, summary, versions, policy, extra_perms)
VALUES (2002, 'test-workspace', 'f/shared/shared_app', 'Shared app', '{}',
        '{"on_behalf_of": "u/admin", "on_behalf_of_email": "admin@windmill.dev", "execution_mode": "viewer"}', '{}');

-- Create app versions
INSERT INTO app_version (id, app_id, value, created_by, created_at)
VALUES
    (2001, 2001, '{"grid": []}', 'alice', NOW()),
    (2002, 2002, '{"grid": []}', 'admin', NOW());

-- Update apps with version references
UPDATE app SET versions = ARRAY[2001::bigint] WHERE id = 2001;
UPDATE app SET versions = ARRAY[2002::bigint] WHERE id = 2002;
