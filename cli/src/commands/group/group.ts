import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import * as wmill from "../../../gen/services.gen.ts";

async function list(opts: GlobalOptions & { json?: boolean }) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const groups = await wmill.listGroups({
    workspace: workspace.workspaceId,
  });

  if (opts.json) {
    console.log(JSON.stringify(groups));
  } else {
    if (groups.length === 0) {
      log.info("No groups found.");
      return;
    }
    new Table()
      .header(["Name", "Summary", "Members"])
      .padding(2)
      .border(true)
      .body(
        groups.map((g) => [
          g.name,
          g.summary ?? "-",
          String(g.members?.length ?? 0),
        ])
      )
      .render();
  }
}

async function get(
  opts: GlobalOptions & { json?: boolean },
  name: string
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const group = await wmill.getGroup({
    workspace: workspace.workspaceId,
    name,
  });

  if (opts.json) {
    console.log(JSON.stringify(group));
  } else {
    console.log(colors.bold("Name:") + " " + group.name);
    console.log(colors.bold("Summary:") + " " + (group.summary ?? "-"));
    console.log(
      colors.bold("Members:") +
        " " +
        (group.members && group.members.length > 0
          ? group.members.join(", ")
          : "(none)")
    );
  }
}

async function create(
  opts: GlobalOptions & { summary?: string },
  name: string
) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await wmill.createGroup({
    workspace: workspace.workspaceId,
    requestBody: {
      name,
      summary: opts.summary,
    },
  });

  log.info(colors.green(`Group '${name}' created.`));
}

async function deleteGroup(opts: GlobalOptions, name: string) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await wmill.deleteGroup({
    workspace: workspace.workspaceId,
    name,
  });

  log.info(colors.green(`Group '${name}' deleted.`));
}

async function addUser(opts: GlobalOptions, name: string, username: string) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await wmill.addUserToGroup({
    workspace: workspace.workspaceId,
    name,
    requestBody: { username },
  });

  log.info(colors.green(`User '${username}' added to group '${name}'.`));
}

async function removeUser(opts: GlobalOptions, name: string, username: string) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await wmill.removeUserToGroup({
    workspace: workspace.workspaceId,
    name,
    requestBody: { username },
  });

  log.info(colors.green(`User '${username}' removed from group '${name}'.`));
}

const command = new Command()
  .description("Manage workspace groups")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "List all groups in the workspace")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "Get group details and members")
  .arguments("<name:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("create", "Create a new group")
  .arguments("<name:string>")
  .option("--summary <summary:string>", "Group summary/description")
  .action(create as any)
  .command("delete", "Delete a group")
  .arguments("<name:string>")
  .action(deleteGroup as any)
  .command("add-user", "Add a user to a group")
  .arguments("<name:string> <username:string>")
  .action(addUser as any)
  .command("remove-user", "Remove a user from a group")
  .arguments("<name:string> <username:string>")
  .action(removeUser as any);

export default command;
