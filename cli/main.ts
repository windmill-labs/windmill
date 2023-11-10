import {
  Command,
  CompletionsCommand,
  DenoLandProvider,
  UpgradeCommand,
  log,
} from "./deps.ts";
import flow from "./flow.ts";
import app from "./apps.ts";
import script from "./script.ts";
import workspace from "./workspace.ts";
import resource from "./resource.ts";
import user from "./user.ts";
import variable from "./variable.ts";
import push from "./push.ts";
import pull from "./pull.ts";
import hub from "./hub.ts";
import folder from "./folder.ts";
import schedule from "./schedule.ts";
import sync from "./sync.ts";
import dev from "./dev.ts";
import { tryResolveVersion } from "./context.ts";
import { GlobalOptions } from "./types.ts";
import { OpenAPI } from "./deps.ts";
import { getHeaders } from "./utils.ts";

addEventListener("error", (event) => {
  if (event.error) {
    console.error("Error details of: " + event.error.message);
    console.error(JSON.stringify(event.error, null, 4));
  }
});

export const VERSION = "v1.204.1";

let command: any = new Command()
  .name("wmill")
  .action(() =>
    log.info(`Welcome to Windmill CLI ${VERSION}. Use -h for help.`)
  )
  .description("A simple CLI tool for windmill.")

  .globalOption(
    "--workspace <workspace:string>",
    "Specify the target workspace. This overrides the default workspace."
  )
  .globalOption("--debug --verbose", "Show debug/verbose logs")
  .globalOption(
    "--show-diffs",
    "Show diff informations when syncing (may show sensitive informations)"
  )
  .globalOption(
    "--token <token:string>",
    "Specify an API token. This will override any stored token."
  )
  .env(
    "HEADERS <headers:string>",
    "Specify headers to use for all requests. e.g: \"HEADERS='h1: v1, h2: v2'\""
  )
  .version(VERSION)
  .command("app", app)
  .command("flow", flow)
  .command("script", script)
  .command("workspace", workspace)
  .command("resource", resource)
  .command("user", user)
  .command("variable", variable)
  .command("hub", hub)
  .command("folder", folder)
  .command("schedule", schedule)
  .command("dev", dev)
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
    })
  )
  .command("completions", new CompletionsCommand());
if (Number.parseInt(VERSION.replace("v", "").replace(".", "")) > 1700) {
  command = command.command("push", push).command("pull", pull);
}

export let showDiffs = false;
try {
  if (Deno.args.length === 0) {
    command.showHelp();
  }
  const LOG_LEVEL =
    Deno.args.includes("--verbose") || Deno.args.includes("--debug")
      ? "DEBUG"
      : "INFO";
  // const NO_COLORS = Deno.args.includes("--no-colors");
  showDiffs = Deno.args.includes("--show-diffs");

  log.setup({
    handlers: {
      console: new log.handlers.ConsoleHandler(LOG_LEVEL, {
        formatter: "{msg}",
      }),
    },
    loggers: {
      default: {
        level: LOG_LEVEL,
        handlers: ["console"],
      },
    },
  });
  log.debug("Debug logging enabled. CLI build against " + VERSION);

  const extraHeaders = getHeaders();
  if (extraHeaders) {
    OpenAPI.HEADERS = extraHeaders;
  }
  await command.parse(Deno.args);
} catch (e) {
  if (e.name === "ApiError") {
    console.log("Server failed. " + e.statusText + ": " + e.body);
  }
  throw e;
}

export default command;
