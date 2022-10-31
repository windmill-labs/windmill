import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/mod.ts";
import {
  DenoLandProvider,
  UpgradeCommand,
} from "https://deno.land/x/cliffy@v0.25.4/command/upgrade/mod.ts";
import login from "./login.ts";
import flow from "./flow.ts";
import script from "./script.ts";

await new Command()
  .name("windmill")
  .description("A simple CLI tool for windmill.")
  .globalOption(
    "--base-url <baseUrl:string>",
    "Specify the base url to use when interacting with the API.",
    {
      default: "https://app.windmill.dev/",
    }
  )
  .version("v0.0.0")
  .command("login", login)
  .command("flow", flow)
  .command("script", script)
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
      provider: new DenoLandProvider(),
    })
  )
  .parse(Deno.args);
