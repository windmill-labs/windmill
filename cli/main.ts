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
import {
  JobService,
  setClient,
} from "https://deno.land/x/windmill@v1.50.0/mod.ts";
import { requireLogin } from "./context.ts";

const VERSION = "v1.52.0";

setClient("gQAYI4FyuNdOvfcBVkw4bGmyTDn4sP", "https://app.windmill.dev");
const list = await JobService.listQueue({ workspace: "npm-demos" });
const requests = [];
for (const j of list) {
  console.log("cancelling " + j.id);
  requests.push(JobService.cancelQueuedJob({
    workspace: "npm-demos",
    id: j.id,
    requestBody: {},
  }));
}
console.log("waiting for completion");
await Promise.all(requests);
console.log("done");

const command = new Command()
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
  );

try {
  await command.parse(Deno.args);
} catch (e) {
  if (e.name === "ApiError") {
    console.log("Server failed. " + e.statusText + ": " + e.body);
  } else {
    throw e;
  }
}

export default command;
