// deno-lint-ignore-file no-explicit-any
import { requireLogin } from "./context.ts";
import {
  GlobalOptions,
  isSuperset,
  removeType,
  removePathPrefix,
} from "./types.ts";
import {
  colors,
  Command,
  GlobalUserInfo,
  log,
  passwordGenerator,
  Table,
  UserService,
  GroupService,
  WorkspaceService,
  GranularAclService,
} from "./deps.ts";

async function list(opts: GlobalOptions) {
  await requireLogin(opts);

  const perPage = 10;
  let page = 0;
  const total: GlobalUserInfo[] = [];
  while (true) {
    const res = await UserService.listUsersAsSuperAdmin({ page, perPage });
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
  const password_final = password ?? passwordGenerator("*", 15);
  await UserService.createUserGlobally({
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

  await UserService.globalUserDelete({ email });
  log.info(colors.green("Deleted User " + email));
}

async function createToken(
  opts: GlobalOptions & { email: string; password: string }
) {
  if (opts.email && opts.password) {
    log.info(
      "Token: " +
        (await UserService.login({
          requestBody: {
            email: opts.email,
            password: opts.password,
          },
        }))
    );
  }

  await requireLogin(opts);
  log.info("Token: " + (await UserService.createToken({ requestBody: {} })));
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
    const remoteUser = await UserService.getUser({
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

  if (user) {
    if (isSuperset(localUser, user)) {
      log.debug(`User ${email} is up to date`);
      return;
    }
    log.debug(`User ${email} is not up-to-date, updating...`);
    try {
      await UserService.updateUser({
        workspace: workspace,
        username: localUser.username,
        requestBody: {
          is_admin: localUser.role === "admin",
          operator: localUser.role === "operator",
          disabled: localUser.disabled,
        },
      });
    } catch (e) {
      console.error(e.body);
      throw e;
    }
  } else {
    console.log(colors.bold.yellow("Creating new user: " + email));
    try {
      await WorkspaceService.addUser({
        workspace: workspace,
        requestBody: {
          email: email,
          is_admin: localUser.role === "admin",
          operator: localUser.role === "operator",
          username: localUser.username,
        },
      });
    } catch (e) {
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
    const remoteGroup = await GroupService.getGroup({
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
      await GroupService.updateGroup({
        workspace: workspace,
        name,
        requestBody: {
          summary: localGroup.summary,
        },
      });
    } catch (e) {
      console.error(e.body);
      throw e;
    }

    for (const member of [...localGroup.members, ...localGroup.admins]) {
      try {
        if ([...group.members, ...group.admins].includes(member)) {
          log.debug(`${member} is already in group ${name}`);
        } else {
          log.debug(`Adding ${member} to group ${name}`);
          await GroupService.addUserToGroup({
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
          await GranularAclService.addGranularAcls({
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
          await GranularAclService.addGranularAcls({
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
          await GroupService.removeUserToGroup({
            workspace: workspace,
            name,
            requestBody: {
              username: member.slice(2),
            },
          });

          await GranularAclService.removeGranularAcls({
            workspace: workspace,
            kind: "group_",
            path: name,
            requestBody: {
              owner: member,
            },
          });
        } catch (e) {
          console.error(e.body);
          throw e;
        }
      }
    }
  } else {
    console.log(colors.bold.yellow("Creating new user: " + name));
    try {
      await GroupService.createGroup({
        workspace: workspace,
        requestBody: {
          name,
          summary: localGroup.summary,
        },
      });

      for (const member of [...localGroup.members, ...localGroup.admins]) {
        log.debug(`Adding user ${member} to group ${name}`);
        try {
          await GroupService.addUserToGroup({
            workspace: workspace,
            name,
            requestBody: {
              username: member.slice(2),
            },
          });
          if (localGroup.admins.includes(member)) {
            log.debug(`Setting role of ${member} as admin in group ${name}`);
            await GranularAclService.addGranularAcls({
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
          console.error(e.body);
          throw e;
        }
      }
    } catch (e) {
      console.error(e.body);
      throw e;
    }
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
