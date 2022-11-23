import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import {
  Input,
  Select,
  Secret,
  prompt,
} from "https://deno.land/x/cliffy@v0.25.4/prompt/mod.ts";
import {
  setClient,
  UserService,
  WorkspaceService,
} from "https://deno.land/x/windmill@v1.50.0/mod.ts";
import { getStore } from "./store.ts";
import { add as addRemote, setDefault as setDefaultRemote } from "./remote.ts";

async function setup(_opts: never) {
  let baseUrl = await Input.prompt({
    message: "What's your base url?",
    suggestions: ["http://app.windmill.dev", "http://localhost"],
    default: "http://app.windmill.dev",
  });
  const remoteName = await Input.prompt({
    message: "What do you want to name this remote?",
    suggestions: ["origin", "local", "cloud"],
    default:
      baseUrl == "http://localhost"
        ? "local"
        : baseUrl == "http://app.windmill.dev"
        ? "cloud"
        : undefined,
  });
  if (baseUrl.endsWith("/")) baseUrl = baseUrl.substring(0, baseUrl.length - 1);
  setClient("no-token", baseUrl);
  const m = await Select.prompt({
    message: "Login Method",
    options: [
      {
        name: "Username/Password",
        value: "up",
      },
      {
        name: "Token",
        value: "t",
      },
    ],
  });
  let token: string;
  if (m == "up") {
    const { email, password } = await prompt([
      {
        name: "email",
        type: Input,
        message: "What's your email?",
        after: async ({ email }, next) => {
          if (email) await next();
          else await next("email");
        },
      },
      {
        name: "password",
        type: Secret,
        message: "What's your password?",
        after: async ({ password }, next) => {
          if (password) await next();
          else await next("password");
        },
      },
    ]);
    token = await UserService.login({
      requestBody: {
        email: email!,
        password: password!,
      },
    });
  } else {
    token = await Secret.prompt("What's your token?");
  }
  setClient(token, baseUrl);
  const workspaces = await WorkspaceService.listWorkspaces();
  const urlStore = await getStore(baseUrl);
  const defaultWorkspaceId = await Select.prompt({
    message: "Select a default workspace",
    options: workspaces.map((x) => ({ name: x.name, value: x.id })),
  });
  await addRemote(_opts, remoteName, baseUrl);
  await setDefaultRemote(_opts, remoteName);
  await Deno.writeTextFile(urlStore + "token", token);
  await Deno.writeTextFile(
    urlStore + "default_workspace_id",
    defaultWorkspaceId
  );
  console.log(colors.green.bold.underline("Everything setup. Ready to use."));
}

const command = new Command()
  .description("setup windmill access")
  .action(setup as any);

export default command;
