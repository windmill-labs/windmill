import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/mod.ts";
import {
  DenoLandProvider,
  UpgradeCommand,
} from "https://deno.land/x/cliffy@v0.25.4/command/upgrade/mod.ts";
import flow from "./flow.ts";
import script from "./script.ts";
import workspace from "./workspace.ts";
import resource from "./resource.ts";
import user from "./user.ts";
import variable from "./variable.ts";
import push from "./push.ts";
import pull from "./pull.ts";
import hub from "./hub.ts";

const VERSION = "v1.53.0";

try {
  await new Command()
    .name("wmill")
    .description("A simple CLI tool for windmill.")
    .globalOption(
      "--workspace <workspace:string>",
      "Specify the target workspace. This overrides the default workspace.",
    )
    .version(VERSION)
    .command("flow", flow)
    .command("script", script)
    .command("workspace", workspace)
    .command("resource", resource)
    .command("user", user)
    .command("variable", variable)
    .command("push", push)
    .command("pull", pull)
    .command("hub", hub)
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
    )
    .parse(Deno.args);
} catch (e) {
  if (e.name === "ApiError") {
    console.log("Server failed. " + e.statusText + ": " + e.body);
  } else {
    throw e;
  }
}
