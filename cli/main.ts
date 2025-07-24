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
import trigger from "./trigger.ts";
import sync from "./sync.ts";
import gitsyncSettings from "./gitsync-settings.ts";
import instance from "./instance.ts";
import workerGroups from "./worker_groups.ts";
import { SCRIPT_GUIDANCE } from "./script_guidance.ts";

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
import { readLockfile } from "./metadata.ts";

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
    trigger,
    sync,
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

export const VERSION = "1.512.0";

const command = new Command()
    .name("wmill")
    .action(() =>
        log.info(`Welcome to Windmill CLI ${VERSION}. Use -h for help.`),
    )
    .description("Windmill CLI")

    .globalOption(
        "--workspace <workspace:string>",
        "Specify the target workspace. This overrides the default workspace.",
    )
    .globalOption("--debug --verbose", "Show debug/verbose logs")
    .globalOption(
        "--show-diffs",
        "Show diff informations when syncing (may show sensitive informations)",
    )
    .globalOption(
        "--token <token:string>",
        "Specify an API token. This will override any stored token.",
    )
    .globalOption(
        "--base-url <baseUrl:string>",
        "Specify the base URL of the API. If used, --token and --workspace are required and no local remote/workspace already set will be used.",
    )
    .globalOption(
        "--config-dir <configDir:string>",
        "Specify a custom config directory. Overrides WMILL_CONFIG_DIR environment variable and default ~/.config location.",
    )
    .env(
        "HEADERS <headers:string>",
        "Specify headers to use for all requests. e.g: \"HEADERS='h1: v1, h2: v2'\"",
    )
    .version(VERSION)
    .versionOption(false)
    .command("init", "Bootstrap a windmill project with a wmill.yaml file")
    .option("--use-default", "Use default settings without checking backend")
    .option("--use-backend", "Use backend git-sync settings if available")
    .option(
        "--repository <repo:string>",
        "Specify repository path (e.g., u/user/repo) when using backend settings",
    )
    .action(
        async (
            opts: {
                useDefault?: boolean;
                useBackend?: boolean;
                repository?: string;
                workspace?: string;
                debug?: unknown;
                showDiffs?: boolean;
                token?: string;
                baseUrl?: string;
                configDir?: string;
            },
        ) => {
            if (await Deno.stat("wmill.yaml").catch(() => null)) {
                log.error(colors.red("wmill.yaml already exists"));
            } else {
                // Import DEFAULT_SYNC_OPTIONS from conf.ts
                const { DEFAULT_SYNC_OPTIONS } = await import("./conf.ts");

                // Create initial config with defaults
                const initialConfig = {
                    defaultTs: DEFAULT_SYNC_OPTIONS.defaultTs,
                    includes: DEFAULT_SYNC_OPTIONS.includes,
                    excludes: DEFAULT_SYNC_OPTIONS.excludes,
                    codebases: DEFAULT_SYNC_OPTIONS.codebases,
                    skipVariables: DEFAULT_SYNC_OPTIONS.skipVariables,
                    skipResources: DEFAULT_SYNC_OPTIONS.skipResources,
                    skipSecrets: DEFAULT_SYNC_OPTIONS.skipSecrets,
                    skipScripts: DEFAULT_SYNC_OPTIONS.skipScripts,
                    skipFlows: DEFAULT_SYNC_OPTIONS.skipFlows,
                    skipApps: DEFAULT_SYNC_OPTIONS.skipApps,
                    skipFolders: DEFAULT_SYNC_OPTIONS.skipFolders,
                    includeSchedules: DEFAULT_SYNC_OPTIONS.includeSchedules,
                    includeTriggers: DEFAULT_SYNC_OPTIONS.includeTriggers,
                    overrides: {},
                };

                await Deno.writeTextFile(
                    "wmill.yaml",
                    yamlStringify(initialConfig),
                );
                log.info(
                    colors.green("wmill.yaml created with default settings"),
                );

                // Create lock file
                await readLockfile();

                // Check for backend git-sync settings unless --use-default is specified
                if (!opts.useDefault) {
                    try {
                        const { requireLogin } = await import("./auth.ts");
                        const { resolveWorkspace } = await import("./context.ts");

                        // Check if user has workspace configured
                        const { getActiveWorkspace } = await import(
                            "./workspace.ts"
                        );
                        const activeWorkspace = await getActiveWorkspace(opts as GlobalOptions);

                        if (!activeWorkspace) {
                            log.info(
                                "No workspace configured. Using default settings.",
                            );
                            log.info(
                                "You can configure a workspace later with 'wmill workspace add'",
                            );
                            return;
                        }

                        await requireLogin(opts as GlobalOptions);
                        const workspace = await resolveWorkspace(opts as GlobalOptions);

                        const wmill = await import("./gen/services.gen.ts");
                        const settings = await wmill.getSettings({
                            workspace: workspace.workspaceId,
                        });

                        if (
                            settings.git_sync?.repositories &&
                            settings.git_sync.repositories.length > 0
                        ) {
                            let useBackendSettings = opts.useBackend;

                            // If repository is specified, implicitly use backend settings
                            if (opts.repository && !opts.useDefault) {
                                useBackendSettings = true;
                            }

                            if (useBackendSettings === undefined) {
                                // Interactive prompt
                                const { Select } = await import("./deps.ts");
                                const choice = await Select.prompt({
                                    message:
                                        "Git-sync settings found on backend. What would you like to do?",
                                    options: [
                                        {
                                            name: "Use backend git-sync settings",
                                            value: "backend",
                                        },
                                        {
                                            name: "Use default settings",
                                            value: "default",
                                        },
                                        {
                                            name: "Cancel",
                                            value: "cancel",
                                        },
                                    ],
                                });

                                if (choice === "cancel") {
                                    // Clean up the created files
                                    try {
                                        await Deno.remove("wmill.yaml");
                                        await Deno.remove("wmill-lock.yaml");
                                    } catch (e) {
                                        // Ignore cleanup errors
                                    }
                                    log.info("Init cancelled");
                                    Deno.exit(0);
                                }

                                useBackendSettings = choice === "backend";
                            }

                            if (useBackendSettings) {
                                log.info(
                                    "Applying git-sync settings from backend...",
                                );

                                // Import and run the pull git-sync settings logic
                                const { pullGitSyncSettings } = await import(
                                    "./gitsync-settings.ts"
                                );
                                await pullGitSyncSettings({
                                    ...(opts as GlobalOptions),
                                    repository: opts.repository,
                                    jsonOutput: false,
                                    diff: false,
                                    replace: true, // Auto-replace when using backend settings during init
                                });

                                log.info(
                                    colors.green(
                                        "Git-sync settings applied from backend",
                                    ),
                                );
                            }
                        }
                    } catch (error) {
                        // If there's an error checking backend settings, just continue with defaults
                        log.warn(
                            `Could not check backend for git-sync settings: ${error.message}`,
                        );
                        log.info("Continuing with default settings");
                    }
                }
            }

            // Create .cursor/rules directory and files with SCRIPT_GUIDANCE content
            try {
                const scriptGuidanceContent = SCRIPT_GUIDANCE;
                                
                // Create .cursor/rules directory
                await Deno.mkdir(".cursor/rules", { recursive: true });
                
                // Create windmill.mdc file
                if (!await Deno.stat(".cursor/rules/windmill.mdc").catch(() => null)) {
                    await Deno.writeTextFile(".cursor/rules/windmill.mdc", scriptGuidanceContent);
                    log.info(colors.green("Created .cursor/rules/windmill.mdc"));
                }
                
                // Create CLAUDE.md file
                if (!await Deno.stat("CLAUDE.md").catch(() => null)) {
                    await Deno.writeTextFile("CLAUDE.md", scriptGuidanceContent);
                    log.info(colors.green("Created CLAUDE.md"));
                }
                
            } catch (error) {
                if (error instanceof Error) {
                    log.warn(`Could not create guidance files: ${error.message}`);
                } else {
                    log.warn(`Could not create guidance files: ${error}`);
                }
            }

        },
    )
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
    .command("trigger", trigger)
    .command("dev", dev)
    .command("sync", sync)
    .command("gitsync-settings", gitsyncSettings)
    .command("instance", instance)
    .command("worker-groups", workerGroups)
    .command("workers", workers)
    .command("queues", queues)
    .command("version --version", "Show version information")
    .action(async (opts) => {
        console.log("CLI version: " + VERSION);
        try {
            const provider = new NpmProvider({ package: "windmill-cli" });
            const versions = await provider.getVersions("windmill-cli");
            if (versions.latest !== VERSION) {
                console.log(
                    `CLI is outdated. Latest version ${versions.latest} is available. Run \`wmill upgrade\` to update.`,
                );
            } else {
                console.log("CLI is up to date");
            }
        } catch (e) {
            console.warn(
                `Cannot fetch latest CLI version on npmjs to check if up-to-date: ${e}`,
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
                "Cannot fetch backend version: no active workspace selected, choose one to pick a remote to fetch version of",
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
                "Try running with sudo and otherwise check the result of the command: npm uninstall windmill-cli && npm install -g windmill-cli",
            );
        }),
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
                if (Deno.env.get("SKIP_DENO_DEPRECATION_WARNING") !== "true") {
                    log.warn(
                        "Using the deno runtime for the Windmill CLI is deprecated, you can now use node: deno uninstall wmill && npm install -g windmill-cli. To skip this warning set SKIP_DENO_DEPRECATION_WARNING=true",
                    );
                }
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
