#!/usr/bin/env node
import "./_dnt.polyfills.js";
import * as dntShim from "./_dnt.shims.js";
import { Command, CompletionsCommand, UpgradeCommand, colors, esMain, log, yamlStringify, } from "./deps.js";
import flow from "./flow.js";
import app from "./apps.js";
import script from "./script.js";
import workspace, { getActiveWorkspace } from "./workspace.js";
import resource from "./resource.js";
import user from "./user.js";
import variable from "./variable.js";
import push from "./push.js";
import pull from "./pull.js";
import hub from "./hub.js";
import folder from "./folder.js";
import schedule from "./schedule.js";
import sync from "./sync.js";
import instance from "./instance.js";
import dev from "./dev.js";
import { fetchVersion } from "./context.js";
import { OpenAPI } from "./deps.js";
import { getHeaders } from "./utils.js";
import { NpmProvider } from "./upgrade.js";
// addEventListener("error", (event) => {
//   if (event.error) {
//     console.error("Error details of: " + event.error.message);
//     console.error(JSON.stringify(event.error, null, 4));
//   }
// });
export const VERSION = "v1.391.0";
let command = new Command()
    .name("wmill")
    .action(() => log.info(`Welcome to Windmill CLI ${VERSION}. Use -h for help.`))
    .description("A simple CLI tool for windmill.")
    .globalOption("--workspace <workspace:string>", "Specify the target workspace. This overrides the default workspace.")
    .globalOption("--debug --verbose", "Show debug/verbose logs")
    .globalOption("--show-diffs", "Show diff informations when syncing (may show sensitive informations)")
    .globalOption("--token <token:string>", "Specify an API token. This will override any stored token.")
    .env("HEADERS <headers:string>", "Specify headers to use for all requests. e.g: \"HEADERS='h1: v1, h2: v2'\"")
    .version(VERSION)
    .command("init", "Bootstrap a windmill project with a wmill.yaml file")
    .action(async () => {
    if (await dntShim.Deno.stat("wmill.yaml").catch(() => null)) {
        log.error(colors.red("wmill.yaml already exists"));
        return;
    }
    await dntShim.Deno.writeTextFile("wmill.yaml", yamlStringify({
        defaultTs: "bun",
        includes: ["**"],
        excludes: [],
        codebases: [],
    }));
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
    .command("version", "Show version information")
    .action(async (opts) => {
    console.log("CLI build against " + VERSION);
    const workspace = await getActiveWorkspace(opts);
    if (workspace) {
        const backendVersion = await fetchVersion(workspace.remote);
        console.log("Backend Version: " + backendVersion);
    }
    else {
        console.log("Cannot fetch backend version: no active workspace selected, choose one to pick a remote to fetch version of");
    }
})
    .command("upgrade", new UpgradeCommand({
    provider: new NpmProvider({ package: "windmill-cli" }),
}))
    .command("completions", new CompletionsCommand());
if (Number.parseInt(VERSION.replace("v", "").replace(".", "")) > 1700) {
    command = command.command("push", push).command("pull", pull);
}
export let showDiffs = false;
async function main() {
    try {
        if (dntShim.Deno.args.length === 0) {
            command.showHelp();
        }
        const LOG_LEVEL = dntShim.Deno.args.includes("--verbose") || dntShim.Deno.args.includes("--debug")
            ? "DEBUG"
            : "INFO";
        // const NO_COLORS = Deno.args.includes("--no-colors");
        showDiffs = dntShim.Deno.args.includes("--show-diffs");
        log.setup({
            handlers: {
                console: new log.ConsoleHandler(LOG_LEVEL, {
                    formatter: ({ msg }) => `${msg}`,
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
        await command.parse(dntShim.Deno.args);
    }
    catch (e) {
        if (e.name === "ApiError") {
            console.log("Server failed. " + e.statusText + ": " + e.body);
        }
        throw e;
    }
}
//@ts-ignore
if (esMain.default(import.meta)) {
    main();
    // test1();
    // test2();
    // module was not imported but called directly
}
// function test1() {
//   // dnt-shim-ignore deno-lint-ignore no-explicit-any
//   const { Deno, process } = globalThis as any;
//   console.log(Deno);
// }
// function test2() {
//   const { Deno, process } = globalThis as any;
//   console.log(Deno);
// }
export default command;
