import {
  Command,
  CompletionsCommand,
  UpgradeCommand,
  colors,
  esMain,
  log,
  yamlStringify,
} from "./deps.ts";
import flow from "./flow.ts";
import app from "./apps.ts";
import script from "./script.ts";
import workspace, { getActiveWorkspace } from "./workspace.ts";
import resource from "./resource.ts";
import user from "./user.ts";
import variable from "./variable.ts";
import hub from "./hub.ts";
import folder from "./folder.ts";
import schedule from "./schedule.ts";
import sync from "./sync.ts";
import instance from "./instance.ts";
import workerGroups from "./worker_groups.ts";

import dev from "./dev.ts";
import { fetchVersion } from "./context.ts";
import { GlobalOptions } from "./types.ts";
import { OpenAPI } from "./gen/index.ts";
import { getHeaders } from "./utils.ts";
import { NpmProvider } from "./upgrade.ts";
import { pull as hubPull } from "./hub.ts";
import { pull, push } from "./sync.ts";
import { add as workspaceAdd } from "./workspace.ts";
import workers from "./workers.ts";
import queues from "./queues.ts";

export {
  flow,
  app,
  script,
  workspace,
  resource,
  user,
  variable,
  hub,
  folder,
  schedule,
  sync,
  instance,
  dev,
  hubPull,
  pull,
  push,
  workspaceAdd,
};

// addEventListener("error", (event) => {
//   if (event.error) {
//     console.error("Error details of: " + event.error.message);
//     console.error(JSON.stringify(event.error, null, 4));
//   }
// });

export const VERSION = "1.403.0";

const command = new Command()
  .name("wmill")
  .action(() =>
    log.info(`Welcome to Windmill CLI ${VERSION}. Use -h for help.`)
  )
  .description("Windmill CLI")

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
  .globalOption(
    "--base-url <baseUrl:string>",
    "Specify the base URL of the API. If used, --token and --workspace are required and no local remote/workspace already set will be used."
  )
  .env(
    "HEADERS <headers:string>",
    "Specify headers to use for all requests. e.g: \"HEADERS='h1: v1, h2: v2'\""
  )
  .version(VERSION)
  .command("init", "Bootstrap a windmill project with a wmill.yaml file")
  .action(async () => {
    if (await Deno.stat("wmill.yaml").catch(() => null)) {
      log.error(colors.red("wmill.yaml already exists"));
      return;
    }
    await Deno.writeTextFile(
      "wmill.yaml",
      yamlStringify({
        defaultTs: "bun",
        includes: ["**"],
        excludes: [],
        codebases: [],
      })
    );
    log.info(colors.green("wmill.yaml created"));
  })
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
  .command("instance", instance)
  .command("worker-groups", workerGroups)
  .command("workers", workers)
  .command("queues", queues)
  .command("version", "Show version information")
  .action(async (opts) => {
    console.log("CLI build against " + VERSION);
    const workspace = await getActiveWorkspace(opts as GlobalOptions);
    if (workspace) {
      const backendVersion = await fetchVersion(workspace.remote);
      console.log("Backend Version: " + backendVersion);
    } else {
      console.log(
        "Cannot fetch backend version: no active workspace selected, choose one to pick a remote to fetch version of"
      );
    }
  })
  .command(
    "upgrade",
    new UpgradeCommand({
      provider: new NpmProvider({ package: "windmill-cli" }),
    }).error((e) => {
      log.error(e);
      log.info(
        "Try running with sudo and otherwise check the result of the command: npm uninstall windmill-cli && npm install -g windmill-cli"
      );
    })
  )
  .command("completions", new CompletionsCommand());

export let showDiffs = false;

let isWin: boolean | undefined = undefined;

export async function getIsWin() {
  if (isWin === undefined) {
    const os = await import("node:os");
    isWin = os.platform() === "win32";
  }
  return isWin;
}

async function main() {
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
        console: new log.ConsoleHandler(LOG_LEVEL, {
          formatter: ({ msg }) => `${msg}`,
          useColors: isWin ? false : true,
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
}

function isMain() {
  // dnt-shim-ignore
  const { Deno } = globalThis as any;

  const isDeno = Deno != undefined;

  if (isDeno) {
    const isMain = import.meta.main;
    if (isMain) {
      if (!Deno.args.includes("completions")) {
        log.warn(
          "Using the deno runtime for the Windmill CLI is deprecated, you can now use node: deno uninstall wmill && npm install -g windmill-cli"
        );
      }
    }
    return isMain;
  } else {
    //@ts-ignore
    return esMain.default(import.meta);
  }
}
if (isMain()) {
  main();
}

export default command;
