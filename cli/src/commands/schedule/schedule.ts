import { mkdir, stat, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { stringify as yamlStringify } from "yaml";

import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import * as wmill from "../../../gen/services.gen.ts";

import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "../../types.ts";
import { Schedule } from "../../../gen/types.gen.ts";
import type { PermissionedAsContext } from "../../core/permissioned_as.ts";
import {
  resolvePermissionedAsRule,
  lookupUsernameByEmail,
} from "../../core/permissioned_as.ts";

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
  if (opts.json) log.setSilent(true);
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
    schedule: "0 0 */6 * * *",
    on_failure: "",
    script_path: "",
    args: {},
    timezone: "Etc/UTC",
    is_flow: false,
    enabled: false,
  };
  await mkdir(dirname(filePath), { recursive: true });
  await writeFile(filePath, yamlStringify(template as Record<string, any>), {
    flag: "wx",
    encoding: "utf-8",
  });
  log.info(colors.green(`Created ${filePath}`));
}

async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  if (opts.json) log.setSilent(true);
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
  localSchedule: ScheduleFile,
  permissionedAsContext?: PermissionedAsContext
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

  // Build preserve flags for permissioned_as
  const preserveFields: { permissioned_as?: string; preserve_permissioned_as?: boolean } = {};
  if (permissionedAsContext?.userIsAdminOrDeployer) {
    if (schedule) {
      // Updating: preserve the remote's permissioned_as (u/username format)
      preserveFields.preserve_permissioned_as = true;
      if ((schedule as Schedule).permissioned_as) {
        preserveFields.permissioned_as = (schedule as Schedule).permissioned_as;
        log.info(`Preserving ${(schedule as Schedule).permissioned_as} as permissioned_as for schedule ${path}`);
      }
    } else {
      // Creating: apply defaultPermissionedAs rule if one matches
      const rule = resolvePermissionedAsRule(
        path,
        permissionedAsContext.rules
      );
      if (rule) {
        const username = await lookupUsernameByEmail(
          workspace,
          rule.email,
          permissionedAsContext.emailToUsernameCache
        );
        preserveFields.permissioned_as = `u/${username}`;
        preserveFields.preserve_permissioned_as = true;
        log.info(`Setting schedule ${path} to run permissioned as ${rule.email} (matched rule '${rule.path_pattern}' in wmill.yaml)`);
      }
    }
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
          ...preserveFields,
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
          ...preserveFields,
        },
      });
    } catch (e) {
      console.error((e as any).body);
      throw e;
    }
  }
}

async function enable(opts: GlobalOptions, path: string) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await wmill.setScheduleEnabled({
    workspace: workspace.workspaceId,
    path,
    requestBody: { enabled: true },
  });

  log.info(colors.green(`Schedule ${path} enabled.`));
}

async function disable(opts: GlobalOptions, path: string) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await wmill.setScheduleEnabled({
    workspace: workspace.workspaceId,
    path,
    requestBody: { enabled: false },
  });

  log.info(colors.yellow(`Schedule ${path} disabled.`));
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

async function setPermissionedAs(
  opts: GlobalOptions,
  schedulePath: string,
  email: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const cache = new Map<string, string>();
  const username = await lookupUsernameByEmail(
    workspace.workspaceId,
    email,
    cache
  );

  await wmill.updateSchedule({
    workspace: workspace.workspaceId,
    path: schedulePath,
    requestBody: {
      permissioned_as: `u/${username}`,
      preserve_permissioned_as: true,
    } as any,
  });
  log.info(
    colors.green(
      `Updated permissioned_as for schedule ${schedulePath} to ${email} (username: ${username})`
    )
  );
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
  .action(push as any)
  .command(
    "set-permissioned-as",
    "Set the email (run-as user) for a schedule (requires admin or wm_deployers group)"
  )
  .arguments("<path:string> <email:string>")
  .action(setPermissionedAs as any)
  .command("enable", "Enable a schedule")
  .arguments("<path:string>")
  .action(enable as any)
  .command("disable", "Disable a schedule")
  .arguments("<path:string>")
  .action(disable as any);

export default command;
