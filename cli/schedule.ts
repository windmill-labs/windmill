// deno-lint-ignore-file no-explicit-any
import {
  colors,
  Command,
  Schedule,
  ScheduleService,
  log,
  Table,
} from "./deps.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "./types.ts";

export interface ScheduleFile {
  schedule: string;
  on_failure: string;
  script_path: string;
  args: any;
  timezone: string;
  is_flow: boolean;
}

async function list(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const schedules = await ScheduleService.listSchedules({
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
  path = removeType(path, "schedule");

  log.debug(`Processing local schedule ${path}`);

  // deleting old app if it exists in raw mode
  try {
    schedule = await ScheduleService.getSchedule({ workspace, path });
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
    log.debug(`Schedule ${path} is not up-to-date, updating...`);
    try {
      await ScheduleService.updateSchedule({
        workspace: workspace,
        path,
        requestBody: {
          ...localSchedule,
        },
      });
    } catch (e) {
      console.error(e.body);
      throw e;
    }
  } else {
    console.log(colors.bold.yellow("Creating new schedule: " + path));
    try {
      await ScheduleService.createSchedule({
        workspace: workspace,
        requestBody: {
          path: path,
          ...localSchedule,
        },
      });
    } catch (e) {
      console.error(e.body);
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

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
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
