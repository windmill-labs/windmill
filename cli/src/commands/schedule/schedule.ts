import { stat } from "node:fs/promises";

import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";
import { SEPARATOR as SEP } from "@std/path";
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

async function list(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const schedules = await wmill.listSchedules({
    workspace: workspace.workspaceId,
  });

  new Table()
    .header(["Path", "Schedule"])
    .padding(2)
    .border(true)
    .body(schedules.map((x) => [x.path, x.schedule]))
    .render();
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
  .action(list as any)
  .command(
    "push",
    "push a local schedule spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
