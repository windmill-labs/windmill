INSERT INTO script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'admins',
'system',
'import wmill from "https://deno.land/x/wmill@v1.53.0/main.ts";

export async function main() {
  await run(
    "workspace", "add", "__automation", "starter", Deno.env.get("WM_BASE_URL") + "/", "--token", Deno.env.get("WM_TOKEN"));

  await run("hub", "pull");
}

async function run(...cmd: string[]) {
  console.log("Running \"" + cmd.join('' '') + "\"");
  await wmill.parse(cmd);
}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'Syncronize Hub Resource types with starter workspace',
'Basic administrative script to sync latest resource types from hub. Recommended to run at least once. On a schedule by default.',
'u/admin/hub_sync', -28028598712388162, 'deno', '');

INSERT INTO schedule(workspace_id, path, edited_by, schedule, enabled, script_path, is_flow, offset_) VALUES (
    'admins',
    'u/admin/sync_with_hub_nightly',
    'system',
    '0 0 0 * * *',
    true,
    'u/admin/hub_sync',
    false,
    0
);