-- Fixture for testing hub_sync blacklist from workspace dependencies
-- hub_sync and apps are already built-in via migrations

-- Insert a simple Bun script that should be affected by workspace dependencies
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'admins',
'test-user',
'
export async function main() {
    return "Simple bun script";
}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'Simple bun script',
'',
'u/admin/simple_bun', 700001, 'bun', '');
