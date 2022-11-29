import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import {
  Input,
  Select,
  Secret,
} from "https://deno.land/x/cliffy@v0.25.4/prompt/mod.ts";
import {
  setClient,
  WorkspaceService,
} from "https://deno.land/x/windmill@v1.50.0/mod.ts";
import { getStore } from "./store.ts";
import { add as addRemote, setDefault as setDefaultRemote } from "./remote.ts";
import { GlobalOptions } from "./types.ts";
import { browserLogin } from "./login.ts";

async function setup(
  opts: GlobalOptions & {
    remote: string | undefined;
    name: string | undefined;
  }
) {
  let baseUrl =
    opts.remote ??
    (await Input.prompt({
      message: "What's your base url?",
      suggestions: ["http://app.windmill.dev", "http://localhost"],
      default: "http://app.windmill.dev",
    }));
  const remoteName =
    opts.name ??
    (await Input.prompt({
      message: "What do you want to name this remote?",
      suggestions: ["origin", "local", "cloud"],
      default:
        baseUrl == "http://localhost"
          ? "local"
          : baseUrl == "http://app.windmill.dev"
          ? "cloud"
          : undefined,
    }));
  const parsedUrl = new URL(baseUrl);
  let urlWorkspace: string | undefined = parsedUrl.username;
  if (urlWorkspace != "") {
    parsedUrl.username = "";
    baseUrl = parsedUrl.toString();
  } else {
    urlWorkspace = undefined;
  }
  if (baseUrl.endsWith("/")) baseUrl = baseUrl.substring(0, baseUrl.length - 1);
  setClient("no-token", baseUrl);
  const m = await Select.prompt({
    message: "Login Method",
    options: [
      {
        name: "Browser",
        value: "br",
      },
      {
        name: "Token",
        value: "t",
      },
    ],
  });
  let token: string | null;
  let workspace: string | null = urlWorkspace ?? null;
  if (m == "br") {
    const { token: newToken, workspace: newWorkspace } = await browserLogin(
      baseUrl
    );
    token = newToken;
    if (newWorkspace) {
      if (workspace && newWorkspace && workspace !== newWorkspace) {
        console.log(
          colors.yellow.underline(
            "! Already got workspace information from URL & mismatched with login information. Using " +
              newWorkspace
          )
        );
      }
      workspace = newWorkspace;
    }
  } else {
    token = await Secret.prompt("What's your token?");
  }
  if (!token) {
    console.log(colors.red("Failed to login. Exiting."));
    return;
  }

  setClient(token, baseUrl);

  const urlStore = await getStore(baseUrl);
  await addRemote(opts, remoteName, baseUrl);
  await setDefaultRemote(opts, remoteName);
  await Deno.writeTextFile(urlStore + "token", token);

  if (!workspace) {
    const workspaces = await WorkspaceService.listWorkspaces();
    const defaultWorkspaceId = await Select.prompt({
      message: "Select a default workspace",
      options: workspaces.map((x) => ({ name: x.name, value: x.id })),
    });
    await Deno.writeTextFile(
      urlStore + "default_workspace_id",
      defaultWorkspaceId
    );
  } else {
    await Deno.writeTextFile(urlStore + "default_workspace_id", workspace);
  }
  console.log(colors.green.bold.underline("Everything setup. Ready to use."));
}

const command = new Command()
  .description("setup windmill access")
  .option("--remote <remote:string>", "Specify the remote inline")
  .option("--name <name:string>", "Specify the remote name inline")
  .action(setup as any);

export default command;
