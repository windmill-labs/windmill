-- Add up migration script here
UPDATE script SET content = 'import wmill from "https://deno.land/x/wmill@v1.189.0/main.ts";
export async function main() {
  await run(
    "workspace", "add", "__automation", "admins", Deno.env.get("BASE_INTERNAL_URL") + "/", "--token", Deno.env.get("WM_TOKEN"));

  await run("hub", "pull");
}

async function run(...cmd: string[]) {
  console.log("Running \"" + cmd.join('' '') + "\"");
  await wmill.parse(cmd);
}', summary = 'Synchronize Hub Resource types with instance',
description = 'Basic administrative script to sync latest resource types from hub to share to every workspace. Recommended to run at least once. On a schedule by default.'
WHERE hash = -28028598712388162 AND workspace_id = 'admins';