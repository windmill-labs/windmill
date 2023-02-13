// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { getRootStore } from "./store.ts";
import { loginInteractive, tryGetLoginInfo } from "./login.ts";
import {
  colors,
  Command,
  DelimiterStream,
  Input,
  setClient,
  Table,
  WorkspaceService,
} from "./deps.ts";
import { decoverto, model, property } from "./decoverto.ts";

@model()
export class Workspace {
  @property(() => String)
  remote: string;
  @property(() => String)
  workspaceId: string;
  @property(() => String)
  name: string;
  @property(() => String)
  token: string;

  constructor(
    remote: string,
    workspaceId: string,
    name: string,
    token: string,
  ) {
    this.remote = remote;
    this.workspaceId = workspaceId;
    this.name = name;
    this.token = token;
  }
}

function makeWorkspaceStream(
  readable: ReadableStream<Uint8Array>,
): ReadableStream<Workspace> {
  return readable
    .pipeThrough(new DelimiterStream(new TextEncoder().encode("\n")))
    .pipeThrough(new TextDecoderStream())
    .pipeThrough(
      new TransformStream({
        transform(line, controller) {
          try {
            if (line.length <= 2) {
              return;
            }
            const workspace = decoverto.type(Workspace).rawToInstance(line);
            workspace.remote = new URL(workspace.remote).toString(); // add trailing slash in all cases!
            controller.enqueue(workspace);
          } catch {
            /* ignore */
          }
        },
      }),
    );
}

export async function getWorkspaceStream() {
  const file = await Deno.open((await getRootStore()) + "remotes.ndjson", {
    write: false,
    read: true,
  });
  return makeWorkspaceStream(file.readable);
}

export async function allWorkspaces(): Promise<Workspace[]> {
  try {
    const workspaceStream = await getWorkspaceStream();
    const workspaces: Workspace[] = [];
    for await (const workspace of workspaceStream) {
      workspaces.push(workspace);
    }

    return workspaces;
  } catch (_) {
    return [];
  }
}

async function getActiveWorkspaceName(
  opts: GlobalOptions,
): Promise<string | undefined> {
  if (opts.workspace) {
    return opts.workspace;
  }
  try {
    return await Deno.readTextFile((await getRootStore()) + "/activeWorkspace");
  } catch {
    return undefined;
  }
}

export async function getActiveWorkspace(
  opts: GlobalOptions,
): Promise<Workspace | undefined> {
  const name = await getActiveWorkspaceName(opts);
  if (!name) {
    return undefined;
  }
  return await getWorkspaceByName(name);
}

export async function getWorkspaceByName(
  workspaceName: string,
): Promise<Workspace | undefined> {
  const workspaceStream = await getWorkspaceStream();
  for await (const workspace of workspaceStream) {
    if (workspace.name === workspaceName) {
      return workspace;
    }
  }
  return undefined;
}

async function list(opts: GlobalOptions) {
  const workspaces = await allWorkspaces();
  const activeName = await getActiveWorkspaceName(opts);

  new Table()
    .header(["name", "remote", "workspace id"])
    .padding(2)
    .border(true)
    .body(
      workspaces.map((x) => {
        const a = [x.name, x.remote, x.workspaceId];
        if (x.name === activeName) {
          return a.map((x) => colors.underline(x));
        } else {
          return a;
        }
      }),
    )
    .render();
}

async function switchC(opts: GlobalOptions, workspaceName: string) {
  if (opts.workspace) {
    console.log(
      colors.red.bold(
        "! Workspace needs to be specified as positional argument, not as option.",
      ),
    );
    return;
  }

  const all = await allWorkspaces();
  if (all.findIndex((x) => x.name === workspaceName) === -1) {
    console.log(colors.red.bold("! This workspace name does not exist."));
    return;
  }

  return await Deno.writeTextFile(
    (await getRootStore()) + "/activeWorkspace",
    workspaceName,
  );
}

export async function add(
  opts: GlobalOptions & {
    create: boolean;
    createWorkspaceName: string | undefined;
    createUsername: string;
  },
  workspaceName: string | undefined,
  workspaceId: string | undefined,
  remote: string | undefined,
) {
  if (opts.workspace) {
    console.log(
      colors.red.bold(
        "! Workspace needs to be specified as positional argument, not as option.",
      ),
    );
    return;
  }

  if (!workspaceName) {
    workspaceName = await Input.prompt("Name this workspace:");
  }

  const all = await allWorkspaces();
  if (all.findIndex((x) => x.name === workspaceName) !== -1) {
    console.log(colors.red.bold("! Workspace name already exists"));
    return;
  }

  if (!workspaceId) {
    workspaceId = await Input.prompt("Enter the ID of this workspace");
  }

  if (!remote) {
    // first check whether workspaceId is actually a URL
    try {
      const url = new URL(workspaceId);
      workspaceId = url.username;
      url.username = "";
      remote = url.toString();
    } catch {
      // not a url
      remote = new URL(
        await Input.prompt({
          message: "Enter the Remote URL",
          suggestions: ["https://app.windmill.dev/"],
        }),
      ).toString();
    }
  }
  remote = new URL(remote).toString(); // add trailing slash in all cases!

  let token = await tryGetLoginInfo(opts);
  while (!token) {
    token = await loginInteractive(remote);
  }

  if (opts.create) {
    setClient(
      token,
      remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote,
    );

    if (
      !await WorkspaceService.existsWorkspace({
        requestBody: { id: workspaceId },
      })
    ) {
      console.log(colors.yellow("Workspace does not exist. Creating..."));
      await WorkspaceService.createWorkspace({
        requestBody: {
          id: workspaceId,
          name: opts.createWorkspaceName ?? workspaceId,
          username: opts.createUsername,
        },
      });
    }
  }

  await addWorkspace({
    name: workspaceName,
    remote: remote,
    workspaceId: workspaceId,
    token: token,
  });
  await Deno.writeTextFile(
    (await getRootStore()) + "/activeWorkspace",
    workspaceName,
  );
  console.log(colors.green.underline("Succesfully added workspace!"));
}

export async function addWorkspace(workspace: Workspace) {
  workspace.remote = new URL(workspace.remote).toString(); // add trailing slash in all cases!
  const file = await Deno.open((await getRootStore()) + "remotes.ndjson", {
    append: true,
    write: true,
    read: false,
    create: true,
  });
  await file.write(new TextEncoder().encode(JSON.stringify(workspace) + "\n"));
  file.close();
}

export async function removeWorkspace(name: string) {
  const orgWorkspaces = await allWorkspaces();
  await Deno.writeTextFile(
    (await getRootStore()) + "remotes.ndjson",
    orgWorkspaces
      .filter((x) => x.name !== name)
      .map((x) => JSON.stringify(x))
      .join("\n"),
  );
}

async function remove(_opts: GlobalOptions, name: string) {
  await removeWorkspace(name);
  console.log(colors.green.underline("Succesfully removed workspace!"));
}

const command = new Command()
  .description("workspace related commands")
  .action(list as any)
  .command("switch")
  .description("Switch to another workspace")
  .arguments("<workspace_name:string>")
  .action(switchC as any)
  .command("add")
  .description("Add a workspace")
  .arguments("[workspace_name:string] [workspace_id:string] [remote:string]")
  .option("-c --create", "Create the workspace if it does not exist")
  .option(
    "--create-workspace-name <workspace_name:string>",
    "Specify the workspace name. Ignored if --create is not specified or the workspace already exists. Will default to the workspace id.",
  )
  .option(
    "--create-username <username:string>",
    "Specify your own username in the newly created workspace. Ignored if --create is not specified or the workspace already exists.",
    {
      default: "admin",
    },
  )
  .action(add as any)
  .command("remove")
  .description("Remove a workspace")
  .arguments("<workspace_name:string>")
  .action(remove as any);

export default command;
