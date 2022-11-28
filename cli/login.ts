import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/mod.ts";
import { GlobalOptions } from "./types.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getStore } from "./store.ts";
import { getDefaultRemote } from "./remote.ts";
import { getAvailablePort } from "https://deno.land/x/port@1.0.0/mod.ts";

export type Options = GlobalOptions;

async function login({
  baseUrl,
  token: existingToken,
  workspace: existingWorkspace,
}: Options) {
  baseUrl = baseUrl ?? (await getDefaultRemote())?.baseUrl;
  baseUrl = baseUrl ?? "https://app.windmill.dev";
  const urlStore = await getStore(baseUrl);

  if (existingToken) {
    await Deno.writeTextFile(urlStore + "token", existingToken);

    if (existingWorkspace) {
      await Deno.writeTextFile(
        urlStore + "default_workspace_id",
        existingWorkspace!
      );
      console.log(
        colors.bold.underline.green(
          "Successfully logged in & switched to workspace " + existingWorkspace
        )
      );
    } else {
      console.log(colors.bold.underline.green("Successfully logged in"));
    }
    return;
  }

  // Start listening on port 8080 of localhost.
  const port = await getAvailablePort();
  if (port == undefined) {
    console.log(colors.red.underline("failed to aquire port"));
    return;
  }

  const server = Deno.listen({ transport: "tcp", port });
  console.log(`Login by going to ${baseUrl}/user/cli?port=${port}`);
  const firstConnection = await server.accept();
  const httpFirstConnection = Deno.serveHttp(firstConnection);
  const firstRequest = (await httpFirstConnection.nextRequest())!;
  const params = new URL(firstRequest.request.url!).searchParams;
  const token = params.get("token");
  const workspace = params.get("workspace");
  await firstRequest?.respondWith(
    Response.redirect(baseUrl + "/user/cli-success", 302)
  );

  if (token === undefined || token === null) {
    console.log(colors.red.underline("Invalid Request. Failed to log in."));
    return;
  }

  await Deno.writeTextFile(urlStore + "token", token);
  await Deno.writeTextFile(urlStore + "default_workspace_id", workspace!);
  console.log(
    colors.bold.underline.green(
      "Successfully logged in & switched to workspace " + workspace
    )
  );

  httpFirstConnection.close();
  server.close();
}

export async function getToken(baseUrl: string): Promise<string> {
  const baseStore = await getStore(baseUrl);
  try {
    return await Deno.readTextFile(baseStore + "token");
  } catch {
    console.log(
      colors.bold.underline.red(
        "You need to be logged in to do this! Run 'windmill login' to login."
      )
    );
    return Deno.exit(-1);
  }
}

const command = new Command()
  .description(
    "Log into windmill. The credentials are not stored, but the token they are exchanged for will be."
  )
  .action(login as any);

export default command;
