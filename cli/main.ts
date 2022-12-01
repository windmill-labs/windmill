import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/mod.ts";
import {
  DenoLandProvider,
  UpgradeCommand,
} from "https://deno.land/x/cliffy@v0.25.4/command/upgrade/mod.ts";
import login from "./login.ts";
import flow from "./flow.ts";
import script from "./script.ts";
import workspace from "./workspace.ts";
import resource from "./resource.ts";
import user from "./user.ts";
import setup from "./setup.ts";
import variable from "./variable.ts";
import push from "./push.ts";
import pull from "./pull.ts";

const VERSION = "v1.51.0";

await new Command()
  .name("wmill")
  .description("A simple CLI tool for windmill.")
  .globalOption(
    "--workspace <workspace_id:string>",
    "Specify the target workspace. This overrides the default workspace."
  )
  .globalOption(
    "--email <email:string>",
    "Specify credentials to use for authentication. This will not be stored. It will only be used to exchange for a token with the API server, which will not be stored either.",
    {
      depends: ["password"],
    }
  )
  .globalOption(
    "--password <password:string>",
    "Specify credentials to use for authentication. This will not be stored. It will only be used to exchange for a token with the API server, which will not be stored either.",
    {
      depends: ["email"],
    }
  )
  .version(VERSION)
  .command("login", login)
  .command("flow", flow)
  .command("script", script)
  .command("workspace", workspace)
  .command("resource", resource)
  .command("user", user)
  .command("setup", setup)
  .command("variable", variable)
  .command("push", push)
  .command("pull", pull)
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
    })
  )
  .parse(Deno.args);
