import { Command, DenoLandProvider, UpgradeCommand } from "./deps.ts";
import flow from "./flow.ts";
import script from "./script.ts";
import workspace from "./workspace.ts";
import resource from "./resource.ts";
import user from "./user.ts";
import variable from "./variable.ts";
import push from "./push.ts";
import pull from "./pull.ts";
import hub from "./hub.ts";
import folder from "./folder.ts";
import sync from "./sync.ts";
import { tryResolveVersion } from "./context.ts";
import { GlobalOptions } from "./types.ts";

const VERSION = "v1.63.2";

let command: any = new Command()
  .name("wmill")
  .description("A simple CLI tool for windmill.")
  .globalOption(
    "--workspace <workspace:string>",
    "Specify the target workspace. This overrides the default workspace.",
  )
  .globalOption(
    "--token <token:string>",
    "Specify an API token. This will override any stored token.",
  )
  .version(VERSION)
  .command("flow", flow)
  .command("script", script)
  .command("workspace", workspace)
  .command("resource", resource)
  .command("user", user)
  .command("variable", variable)
  .command("hub", hub)
  .command("folder", folder)
  .command("sync", sync)
  .command("version", "Show version information")
  .action(async (opts) => {
    console.log("CLI build against " + VERSION);
    const backendVersion = await tryResolveVersion(opts as GlobalOptions);
    if (backendVersion) {
      console.log("Backend Version: " + backendVersion);
    } else {
      console.log("Cannot resolve Backend Version");
    }
  })
  .command(
    "upgrade",
    new UpgradeCommand({
      main: "main.ts",
      args: [
        "--allow-net",
        "--allow-read",
        "--allow-write",
        "--allow-env",
        "--unstable",
      ],
      provider: new DenoLandProvider({ name: "wmill" }),
    }),
  );

if (Number.parseInt(VERSION.replace("v", "").replace(".", "")) > 1700) {
  command = command
    .command("push", push)
    .command("pull", pull);
}

try {
  await command.parse(Deno.args);
} catch (e) {
  if (e.name === "ApiError") {
    console.log("Server failed. " + e.statusText + ": " + e.body);
  }
  throw e;
}

export default command;
