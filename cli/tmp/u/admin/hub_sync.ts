import wmill from "https://deno.land/x/wmill@v1.55.0/main.ts";

export async function main() {
  await run(
    "workspace", "add", "__automation", "starter", Deno.env.get("WM_BASE_URL") + "/", "--token", Deno.env.get("WM_TOKEN"));

  await run("hub", "pull");
}

async function run(...cmd: string[]) {
  console.log("Running \"" + cmd.join(' ') + "\"");
  await wmill.parse(cmd);
}