import { stat, writeFile } from "node:fs/promises";
import { stringify as yamlStringify } from "yaml";

import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";

import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "../../types.ts";
import { Schedule } from "../../../gen/types.gen.ts";

export interface ScheduleFile {
  schedule: string;
  on_failure: string;
  script_path: string;
  args: any;
  timezone: string;
  is_flow: boolean;
  enabled: boolean;
}

async function list(opts: GlobalOptions & { json?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const schedules = await wmill.listSchedules({
    workspace: workspace.workspaceId,
  });

  if (opts.json) {
    console.log(JSON.stringify(schedules));
  } else {
    new Table()
      .header(["Path", "Schedule"])
      .padding(2)
      .border(true)
      .body(schedules.map((x) => [x.path, x.schedule]))
      .render();
  }
}

async function newSchedule(opts: GlobalOptions, path: string) {
  if (!validatePath(path)) {
    return;
  }
  const filePath = path + ".schedule.yaml";
  try {
    await stat(filePath);
    throw new Error("File already exists: " + filePath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
  }
  const template: ScheduleFile = {
    schedule: "0 */6 * * *",
    on_failure: "",
    script_path: "",
    args: {},
    timezone: "Etc/UTC",
    is_flow: false,
    enabled: false,
  };
  await writeFile(filePath, yamlStringify(template as Record<string, any>), {
    flag: "wx",
    encoding: "utf-8",
  });
  log.info(colors.green(`Created ${filePath}`));
}

async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const s = await wmill.getSchedule({
    workspace: workspace.workspaceId,
    path,
  });
  if (opts.json) {
    console.log(JSON.stringify(s));
  } else {
    console.log(colors.bold("Path:") + " " + s.path);
    console.log(colors.bold("Schedule:") + " " + s.schedule);
    console.log(colors.bold("Timezone:") + " " + (s.timezone ?? ""));
    console.log(colors.bold("Script Path:") + " " + (s.script_path ?? ""));
    console.log(colors.bold("Is Flow:") + " " + (s.is_flow ? "true" : "false"));
    console.log(colors.bold("Enabled:") + " " + (s.enabled ? "true" : "false"));
  }
}

export async function pushSchedule(
  workspace: string,
  path: string,
  schedule: Schedule | ScheduleFile | undefined,
  localSchedule: ScheduleFile
): Promise<void> {
  path = removeType(path, "schedule").replaceAll(SEP, "/");
  log.debug(`Processing local schedule ${path}`);

  // deleting old app if it exists in raw mode
  try {
    schedule = await wmill.getSchedule({ workspace, path });
    log.debug(`Schedule ${path} exists on remote`);
  } catch {
    log.debug(`Schedule ${path} does not exist on remote`);
    //ignore
  }

  if (schedule) {
    if (isSuperset(localSchedule, schedule)) {
      log.debug(`Schedule ${path} is up to date`);
      return;
    }
    log.debug(`Updating schedule ${path}`);
    try {
      log.info(colors.bold.yellow(
        `Updating schedule ${path}`
      ));
      await wmill.updateSchedule({
        workspace: workspace,
        path,
        requestBody: {
          ...localSchedule,
        },
      });
      if (localSchedule.enabled != schedule.enabled) {
        log.info(colors.bold.yellow(
          `Schedule ${path} is ${localSchedule.enabled ? "enabled" : "disabled"} locally but not on remote, updating remote`
        ));
        await wmill.setScheduleEnabled({
          workspace: workspace,
          path,
          requestBody: {
            enabled: localSchedule.enabled,
          },
        });
      }
    } catch (e) {
      console.error((e as any).body);
      throw e;
    }
  } else {
    console.log(colors.bold.yellow("Creating new schedule " + path));
    try {
      await wmill.createSchedule({
        workspace: workspace,
        requestBody: {
          path: path,
          ...localSchedule,
        },
      });
    } catch (e) {
      console.error((e as any).body);
      throw e;
    }
  }
}

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing schedule..."));

  await pushSchedule(
    workspace.workspaceId,
    remotePath,
    undefined,
    parseFromFile(filePath)
  );
  console.log(colors.bold.underline.green("Schedule pushed"));
}

const command = new Command()
  .description("schedule related commands")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all schedules")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get a schedule's details")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("new", "create a new schedule locally")
  .arguments("<path:string>")
  .action(newSchedule as any)
  .command(
    "push",
    "push a local schedule spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
