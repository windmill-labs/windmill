import {
  Command,
  CompletionsCommand,
  UpgradeCommand,
  esMain,
  log,
} from "../deps.ts";

// Node.js-specific imports for symlink resolution in isMain()
// These are only used in Node.js, not Deno
// dnt-shim-ignore
import { realpathSync } from "node:fs";
// dnt-shim-ignore
import { fileURLToPath } from "node:url";
import flow from "./commands/flow/flow.ts";
import app from "./commands/app/app.ts";
import script from "./commands/script/script.ts";
import workspace, {
  getActiveWorkspace,
} from "./commands/workspace/workspace.ts";
import resource from "./commands/resource/resource.ts";
import resourceType from "./commands/resource-type/resource-type.ts";
import user from "./commands/user/user.ts";
import variable from "./commands/variable/variable.ts";
import hub from "./commands/hub/hub.ts";
import folder from "./commands/folder/folder.ts";
import schedule from "./commands/schedule/schedule.ts";
import trigger from "./commands/trigger/trigger.ts";
import sync from "./commands/sync/sync.ts";
import gitsyncSettings from "./commands/gitsync-settings/gitsync-settings.ts";
import instance from "./commands/instance/instance.ts";
import workerGroups from "./commands/worker-groups/worker-groups.ts";
import lint from "./commands/lint/lint.ts";

import dev from "./commands/dev/dev.ts";
import { GlobalOptions } from "./types.ts";
import { OpenAPI } from "../gen/index.ts";
import { getHeaders, getIsWin } from "./utils/utils.ts";
import { setShowDiffs } from "./core/conf.ts";
import { NpmProvider } from "./utils/upgrade.ts";
import { pull as hubPull } from "./commands/hub/hub.ts";
import { pull, push } from "./commands/sync/sync.ts";
import { add as workspaceAdd } from "./commands/workspace/workspace.ts";
import workers from "./commands/workers/workers.ts";
import queues from "./commands/queues/queues.ts";
import dependencies from "./commands/dependencies/dependencies.ts";
import init from "./commands/init/init.ts";
import jobs from "./commands/jobs/jobs.ts";
import { fetchVersion } from "./core/context.ts";

export {
  flow,
  app,
  script,
  workspace,
  resource,
  resourceType,
  user,
  variable,
  hub,
  folder,
  schedule,
  trigger,
  sync,
  lint,
  gitsyncSettings,
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

export const VERSION = "1.638.4";

// Re-exported from constants.ts to maintain backwards compatibility
export { WM_FORK_PREFIX } from "./core/constants.ts";

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
  .globalOption(
    "--config-dir <configDir:string>",
    "Specify a custom config directory. Overrides WMILL_CONFIG_DIR environment variable and default ~/.config location."
  )
  .env(
    "HEADERS <headers:string>",
    "Specify headers to use for all requests. e.g: \"HEADERS='h1: v1, h2: v2'\""
  )
  .version(VERSION)
  .versionOption(false)
  .command("init", init)
  .command("app", app)
  .command("flow", flow)
  .command("script", script)
  .command("workspace", workspace)
  .command("resource", resource)
  .command("resource-type", resourceType)
  .command("user", user)
  .command("variable", variable)
  .command("hub", hub)
  .command("folder", folder)
  .command("schedule", schedule)
  .command("trigger", trigger)
  .command("dev", dev)
  .command("sync", sync)
  .command("lint", lint)
  .command("gitsync-settings", gitsyncSettings)
  .command("instance", instance)
  .command("worker-groups", workerGroups)
  .command("workers", workers)
  .command("queues", queues)
  .command("dependencies", dependencies)
  .command("jobs", jobs)
  .command("version --version", "Show version information")
  .action(async (opts: any) => {
    console.log("CLI version: " + VERSION);
    try {
      const provider = new NpmProvider({ package: "windmill-cli" });
      const versions = await provider.getVersions("windmill-cli");
      if (versions.latest !== VERSION) {
        console.log(
          `CLI is outdated. Latest version ${versions.latest} is available. Run \`wmill upgrade\` to update.`
        );
      } else {
        console.log("CLI is up to date");
      }
    } catch (e) {
      console.warn(
        `Cannot fetch latest CLI version on npmjs to check if up-to-date: ${e}`
      );
    }
    const workspace = await getActiveWorkspace(opts as GlobalOptions);
    if (workspace) {
      try {
        const backendVersion = await fetchVersion(workspace.remote);
        console.log("Backend Version: " + backendVersion);
      } catch (e) {
        console.warn("Cannot fetch backend version: " + e);
      }
    } else {
      console.warn(
        "Cannot fetch backend version: no active workspace selected, choose one to pick a remote to fetch version of"
      );
    }
  })
  .command(
    "upgrade",
    new UpgradeCommand({
      provider: new NpmProvider({ package: "windmill-cli" }),
    }).error((e: any) => {
      log.error(e);
      log.info(
        "Try running with sudo and otherwise check the result of the command: npm uninstall windmill-cli && npm install -g windmill-cli"
      );
    })
  )
  .command("completions", new CompletionsCommand());

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
    setShowDiffs(Deno.args.includes("--show-diffs"));

    const isWin = await getIsWin();
    log.setup({
      handlers: {
        console: new log.ConsoleHandler(LOG_LEVEL, {
          formatter: ({ msg }) => msg,
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
    if (e && typeof e === "object" && "name" in e && e.name === "ApiError") {
      console.log(
        "Server failed. " + (e as any).statusText + ": " + (e as any).body
      );
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
        if (Deno.env.get("SKIP_DENO_DEPRECATION_WARNING") !== "true") {
          log.warn(
            "Using the deno runtime for the Windmill CLI is deprecated, you can now use node: deno uninstall wmill && npm install -g windmill-cli. To skip this warning set SKIP_DENO_DEPRECATION_WARNING=true"
          );
        }
      }
    }
    return isMain;
  } else {
    // For Node.js, we need to handle symlinks properly.
    // The dnt polyfill doesn't resolve symlinks when comparing process.argv[1]
    // with import.meta.url, so `wmill` symlink doesn't match the real file path.
    // We resolve symlinks manually to get accurate comparison.
    try {
      const scriptPath = process.argv[1];
      if (!scriptPath) return false;

      const realScriptPath = realpathSync(scriptPath);
      const modulePath = fileURLToPath(import.meta.url);

      return realScriptPath === modulePath;
    } catch {
      // Fallback to esMain if something fails
      //@ts-ignore
      return esMain.default(import.meta);
    }
  }
}
if (isMain()) {
  main();
}

export default command;
