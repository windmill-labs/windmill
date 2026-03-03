import { writeFile } from "node:fs/promises";

import { requireLogin } from "../../core/auth.ts";
import {
  GlobalOptions,
  isSuperset,
  removeType,
  removePathPrefix,
} from "../../types.ts";
import { compareInstanceObjects, InstanceSyncOptions } from "../instance/instance.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { stringify as yamlStringify } from "yaml";
import { yamlParseFile } from "../../utils/yaml.ts";
import * as wmill from "../../../gen/services.gen.ts";
import {
  ExportedInstanceGroup,
  ExportedUser,
  GlobalUserInfo,
  InstanceGroup,
} from "../../../gen/types.gen.ts";

const INSTANCE_USERS_PATH = "instance_users.yaml";
let instanceUsersPath = INSTANCE_USERS_PATH;
function checkInstanceUsersPath(opts: InstanceSyncOptions) {
  if (opts.prefix && opts.folderPerInstance && opts.prefixSettings) {
    instanceUsersPath = `${opts.prefix}/${INSTANCE_USERS_PATH}`;
  }
}

const INSTANCE_GROUPS_PATH = "instance_groups.yaml";
let instanceGroupsPath = INSTANCE_GROUPS_PATH;
function checkInstanceGroupsPath(opts: InstanceSyncOptions) {
  if (opts.prefix && opts.folderPerInstance && opts.prefixSettings) {
    instanceGroupsPath = `${opts.prefix}/${INSTANCE_GROUPS_PATH}`;
  }
}

async function list(opts: GlobalOptions) {
  await requireLogin(opts);

  const perPage = 10;
  let page = 0;
  const total: GlobalUserInfo[] = [];
  while (true) {
    const res = await wmill.listUsersAsSuperAdmin({ page, perPage });
    total.push(...res);
    page += 1;

    if (res.length < perPage) {
      break;
    }
  }
  new Table()
    .header(["email", "name", "company", "verified", "super admin"])
    .padding(2)
    .border(true)
    .body(
      total.map((x) => [
        x.email,
        x.name ?? "-",
        x.company ?? "-",
        x.verified ? "true" : "false",
        x.super_admin ? "true" : "false",
      ])
    )
    .render();
}

function rdString() {
  return Math.random().toString(36).slice(2, 7);
}

async function add(
  opts: GlobalOptions & {
    superadmin?: boolean;
    company?: string;
    name?: string;
  },
  email: string,
  password?: string
) {
  await requireLogin(opts);
  const password_final = password ?? rdString();
  await wmill.createUserGlobally({
    requestBody: {
      email,
      password: password_final,
      super_admin: opts.superadmin ?? false,
      company: opts.company,
      name: opts.name,
    },
  });
  if (!password) {
    log.info(colors.red("New Password: " + password_final));
  }
  log.info(colors.underline.green("New User Created."));
}
async function remove(opts: GlobalOptions, email: string) {
  await requireLogin(opts);

  await wmill.globalUserDelete({ email });
  log.info(colors.green("Deleted User " + email));
}

async function createToken(
  opts: GlobalOptions & { email: string; password: string }
) {
  await requireLogin(opts);

  if (opts.email && opts.password) {
    log.info(
      "Token: " +
        (await wmill.login({
          requestBody: {
            email: opts.email,
            password: opts.password,
          },
        }))
    );
    return;
  }

  log.info("Token: " + (await wmill.createToken({ requestBody: {} })));
}

interface SimplifiedUser {
  role: string;
  username: string;
  disabled: boolean;
}

export async function pushWorkspaceUser(
  workspace: string,
  path: string,
  user: SimplifiedUser | undefined,
  localUser: SimplifiedUser
): Promise<void> {
  const email = removePathPrefix(removeType(path, "user"), "users");

  log.debug(`Processing local user ${email}`);

  if (!["operator", "developer", "admin"].includes(localUser.role)) {
    throw new Error(`Invalid role for user ${email}: ${localUser.role}`);
  }

  try {
    const remoteUser = await wmill.getUser({
      workspace,
      username: localUser.username,
    });
    user = {
      role: remoteUser.is_admin
        ? "admin"
        : remoteUser.operator
        ? "operator"
        : "developer",
      username: remoteUser.username,
      disabled: remoteUser.disabled,
    };
    log.debug(`User ${email} exists on remote`);
  } catch {
    log.debug(`User ${email} does not exist on remote`);
    //ignore
  }

  if (user && user.username !== localUser.username) {
    throw new Error("Username cannot be changed");
  }

  if (user) {
    if (isSuperset(localUser, user)) {
      log.debug(`User ${email} is up to date`);
      return;
    }
    log.debug(`User ${email} is not up-to-date, updating...`);
    try {
      await wmill.updateUser({
        workspace: workspace,
        username: localUser.username,
        requestBody: {
          is_admin: localUser.role === "admin",
          operator: localUser.role === "operator",
          disabled: localUser.disabled,
        },
      });
    } catch (e) {
      //@ts-ignore
      console.error(e.body);
      throw e;
    }
  } else {
    console.log(colors.bold.yellow("Creating new user: " + email));
    try {
      const automatedUsernameCreation: boolean = (await wmill.getGlobal({
        key: "automate_username_creation",
      })) as boolean;
      await wmill.addUser({
        workspace: workspace,
        requestBody: {
          email: email,
          is_admin: localUser.role === "admin",
          operator: localUser.role === "operator",
          username: !automatedUsernameCreation ? localUser.username : undefined,
        },
      });
    } catch (e) {
      //@ts-ignore
      console.error(e.body);
      throw e;
    }
  }
}

interface SimplifiedGroup {
  summary: string | undefined;
  admins: string[];
  members: string[];
}

export async function pushGroup(
  workspace: string,
  path: string,
  group: SimplifiedGroup | undefined,
  localGroup: SimplifiedGroup
): Promise<void> {
  const name = removePathPrefix(removeType(path, "group"), "groups");

  log.debug(`Processing local group ${name}`);

  try {
    const remoteGroup = await wmill.getGroup({
      workspace,
      name,
    });

    // only consider extra_perms that concern actual members of the group
    const admins = Object.entries(remoteGroup.extra_perms ?? {})
      .filter(([k, v]) => v && remoteGroup.members?.includes(k.slice(2)))
      .map(([k, _]) => k)
      .sort();
    group = {
      summary: remoteGroup.summary,
      admins,
      // remove members that are also admins as they are already in the admins list
      members: (remoteGroup.members ?? [])
        .map((m) => "u/" + m)
        .filter((m) => !admins.includes(m)),
    };
    log.debug(`Group ${name} exists on remote`);
  } catch {
    log.debug(`Group ${name} does not exist on remote`);
    //ignore
  }

  if (group) {
    if (isSuperset(localGroup, group)) {
      log.debug(`Group ${name} is up to date`);
      return;
    }
    log.debug(`Group ${name} is not up-to-date, updating...`);
    try {
      await wmill.updateGroup({
        workspace: workspace,
        name,
        requestBody: {
          summary: localGroup.summary,
        },
      });
    } catch (e) {
      //@ts-ignore
      console.error(e.body);
      throw e;
    }

    for (const member of [...localGroup.members, ...localGroup.admins]) {
      try {
        if ([...group.members, ...group.admins].includes(member)) {
          log.debug(`${member} is already in group ${name}`);
        } else {
          log.debug(`Adding ${member} to group ${name}`);
          await wmill.addUserToGroup({
            workspace: workspace,
            name,
            requestBody: {
              username: member.slice(2),
            },
          });
        }
        if (
          localGroup.admins.includes(member) &&
          !group.admins.includes(member)
        ) {
          log.debug(`Setting role of ${member} as admin in group ${name}`);
          await wmill.addGranularAcls({
            workspace: workspace,
            kind: "group_",
            path: name,
            requestBody: {
              owner: member,
              write: true,
            },
          });
        }

        if (
          localGroup.members.includes(member) &&
          !group.members.includes(member)
        ) {
          log.debug(`Setting role of ${member} as member in group ${name}`);
          await wmill.addGranularAcls({
            workspace: workspace,
            kind: "group_",
            path: name,
            requestBody: {
              owner: member,
              write: false,
            },
          });
        }
      } catch (e) {
        //@ts-ignore
        console.error(e.body);
        throw e;
      }
    }

    for (const member of [...group.members, ...group.admins]) {
      if (![...localGroup.members, ...localGroup.admins].includes(member)) {
        log.debug(
          `Removing ${member} and any associated role from group ${name}`
        );
        try {
          await wmill.removeUserToGroup({
            workspace: workspace,
            name,
            requestBody: {
              username: member.slice(2),
            },
          });

          await wmill.removeGranularAcls({
            workspace: workspace,
            kind: "group_",
            path: name,
            requestBody: {
              owner: member,
            },
          });
        } catch (e) {
          //@ts-ignore
          console.error(e.body);
          throw e;
        }
      }
    }
  } else {
    console.log(colors.bold.yellow("Creating new user: " + name));
    try {
      await wmill.createGroup({
        workspace: workspace,
        requestBody: {
          name,
          summary: localGroup.summary,
        },
      });

      for (const member of [...localGroup.members, ...localGroup.admins]) {
        log.debug(`Adding user ${member} to group ${name}`);
        try {
          await wmill.addUserToGroup({
            workspace: workspace,
            name,
            requestBody: {
              username: member.slice(2),
            },
          });
          if (localGroup.admins.includes(member)) {
            log.debug(`Setting role of ${member} as admin in group ${name}`);
            await wmill.addGranularAcls({
              workspace: workspace,
              kind: "group_",
              path: name,
              requestBody: {
                owner: member,
                write: true,
              },
            });
          }
        } catch (e) {
          //@ts-ignore
          console.error(e.body);
          throw e;
        }
      }
    } catch (e) {
      //@ts-ignore
      console.error(e.body);
      throw e;
    }
  }
}

export async function pullInstanceUsers(
  opts: InstanceSyncOptions,
  preview: boolean = false
) {
  const remoteUsers = await wmill.globalUsersExport();

  checkInstanceUsersPath(opts);

  if (preview) {
    const localUsers: ExportedUser[] = await readInstanceUsers(opts);
    return compareInstanceObjects(remoteUsers, localUsers, "email", "user");
  } else {
    log.info("Pulling users from instance...");
    await writeFile(
      instanceUsersPath,
      yamlStringify(remoteUsers as any),
      "utf-8"
    );
    log.info(colors.green(`Users written to ${instanceUsersPath}`));
  }
}

export async function readInstanceUsers(opts: InstanceSyncOptions) {
  let localUsers: ExportedUser[] = [];

  await checkInstanceUsersPath(opts);

  try {
    localUsers = (await yamlParseFile(instanceUsersPath)) as ExportedUser[];
  } catch {
    log.warn(`No ${instanceUsersPath} file found`);
  }
  return localUsers;
}

export async function readInstanceGroups(opts: InstanceSyncOptions) {
  let localGroups: InstanceGroup[] = [];

  checkInstanceGroupsPath(opts);

  try {
    localGroups = (await yamlParseFile(
      instanceGroupsPath
    )) as ExportedInstanceGroup[];
  } catch {
    log.warn(`No ${instanceGroupsPath} file found`);
  }
  return localGroups;
}

export async function pushInstanceUsers(
  opts: InstanceSyncOptions,
  preview: boolean = false
) {
  const remoteUsers = await wmill.globalUsersExport();
  const localUsers: ExportedUser[] = await readInstanceUsers(opts);

  if (preview) {
    return compareInstanceObjects(localUsers, remoteUsers, "email", "user");
  } else {
    log.info("Pushing users to instance...");
    await wmill.globalUsersOverwrite({
      requestBody: localUsers,
    });

    log.info(colors.green("Users pushed to the instance"));
  }
}

export async function pullInstanceGroups(
  opts: InstanceSyncOptions,
  preview = false
) {
  const remoteGroups = await wmill.exportInstanceGroups();

  checkInstanceGroupsPath(opts);

  if (preview) {
    const localGroups = await readInstanceGroups(opts);
    return compareInstanceObjects(remoteGroups, localGroups, "name", "group");
  } else {
    log.info("Pulling groups from instance...");

    await writeFile(
      instanceGroupsPath,
      yamlStringify(remoteGroups as any),
      "utf-8"
    );

    log.info(colors.green(`Groups written to ${instanceGroupsPath}`));
  }
}

export async function pushInstanceGroups(
  opts: InstanceSyncOptions,
  preview: boolean = false
) {
  const remoteGroups = await wmill.exportInstanceGroups();
  const localGroups = await readInstanceGroups(opts);

  if (preview) {
    return compareInstanceObjects(localGroups, remoteGroups, "name", "group");
  } else {
    log.info("Pushing groups to instance...");
    await wmill.overwriteInstanceGroups({
      requestBody: localGroups,
    });

    log.info(colors.green("Groups pushed to the instance"));
  }
}

const command = new Command()
  .description("user related commands")
  .action(list as any)
  .command("add", "Create a user")
  .arguments("<email:string> [password:string]")
  .option("--superadmin", "Specify to make the new user superadmin.")
  .option(
    "--company <company:string>",
    "Specify to set the company of the new user."
  )
  .option("--name <name:string>", "Specify to set the name of the new user.")
  .action(add as any)
  .command("remove", "Delete a user")
  .arguments("<email:string>")
  .action(remove as any)
  .command("create-token")
  .option(
    "--email <email:string>",
    "Specify credentials to use for authentication. This will not be stored. It will only be used to exchange for a token with the API server, which will not be stored either.",
    {
      depends: ["password"],
    }
  )
  .option(
    "--password <password:string>",
    "Specify credentials to use for authentication. This will not be stored. It will only be used to exchange for a token with the API server, which will not be stored either.",
    {
      depends: ["email"],
    }
  )
  .action(createToken as any);

export default command;
