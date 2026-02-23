import { Command } from "@cliffy/command";
import { generateShellCompletions } from "@cliffy/command/completions";
import { UpgradeCommand } from "@cliffy/command/upgrade";
import * as log from "./core/log.ts";

import { realpathSync } from "node:fs";
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
import { getHeaders } from "./utils/utils.ts";
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

export const VERSION = "1.642.0";

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
        console.warn(
          `Cannot fetch backend version from ${workspace.remote} (workspace: ${workspace.name}): ${e}`
        );
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
  .command(
    "completions",
    new Command()
      .description("Generate shell completions.")
      .command("bash", new Command().description("Generate bash completions.").action(() => {
        process.stdout.write(generateShellCompletions(command, "bash") + "\n");
      }))
      .command("zsh", new Command().description("Generate zsh completions.").action(() => {
        process.stdout.write(generateShellCompletions(command, "zsh") + "\n");
      }))
      .command("fish", new Command().description("Generate fish completions.").action(() => {
        process.stdout.write(generateShellCompletions(command, "fish") + "\n");
      }))
  );

async function main() {
  try {
    const args = process.argv.slice(2);
    if (args.length === 0) {
      command.showHelp();
    }
    const LOG_LEVEL =
      args.includes("--verbose") || args.includes("--debug")
        ? "DEBUG"
        : "INFO";
    // const NO_COLORS = args.includes("--no-colors");
    setShowDiffs(args.includes("--show-diffs"));

    log.setup(LOG_LEVEL);
    log.debug("Debug logging enabled. CLI build against " + VERSION);

    const extraHeaders = getHeaders();
    if (extraHeaders) {
      OpenAPI.HEADERS = extraHeaders;
    }
    await command.parse(args);
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
  // Handle symlinks properly: resolve symlinks when comparing process.argv[1]
  // with import.meta.url, so `wmill` symlink matches the real file path.
  try {
    const scriptPath = process.argv[1];
    if (!scriptPath) return false;

    const realScriptPath = realpathSync(scriptPath);
    const modulePath = fileURLToPath(import.meta.url);

    return realScriptPath === modulePath;
  } catch {
    return false;
  }
}
if (isMain()) {
  main().then(() => {
    // Destroy stdin so interactive prompts (Cliffy) don't keep the event loop alive
    process.stdin.destroy();
  });
}

export default command;
