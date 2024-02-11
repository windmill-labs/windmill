import { GlobalOptions } from "./types.ts";
import { colors, getAvailablePort, log, open, Secret, Select } from "./deps.ts";

export async function loginInteractive(remote: string) {
  let token: string | undefined;
  if (Deno.stdin.isTerminal && !Deno.stdin.isTerminal()) {
    log.info("Not a TTY, can't login interactively.");
    return undefined;
  }
  if (
    (await Select.prompt({
      message: "How do you want to login",
      options: [
        {
          name: "Browser",
          value: "b",
        },
        {
          name: "Token",
          value: "t",
        },
      ],
    })) === "b"
  ) {
    token = await browserLogin(remote);
  } else {
    token = await Secret.prompt("Enter your token");
  }

  return token;
}

// deno-lint-ignore require-await
export async function tryGetLoginInfo(
  opts: GlobalOptions
): Promise<string | undefined> {
  if (opts.token) {
    return opts.token;
  }

  return undefined;
}

export async function browserLogin(
  baseUrl: string
): Promise<string | undefined> {
  const port = await getAvailablePort();
  if (port == undefined) {
    log.info(colors.red.underline("failed to aquire port"));
    return undefined;
  }

  const server = Deno.listen({ transport: "tcp", port });
  const url = `${baseUrl}user/cli?port=${port}`;
  log.info(`Login by going to ${url}`);
  try {
    await open(url);
    log.info("Opened browser for you");
  } catch {
    console.error(`Failed to open browser, please navigate to ${url}`);
  }
  const firstConnection = await server.accept();
  const httpFirstConnection = Deno.serveHttp(firstConnection);
  const firstRequest = (await httpFirstConnection.nextRequest())!;
  const params = new URL(firstRequest.request.url!).searchParams;
  const token = params.get("token");
  // const _workspace = params.get("workspace");
  await firstRequest?.respondWith(
    Response.redirect(baseUrl + "user/cli-success", 302)
  );

  setTimeout(() => {
    httpFirstConnection.close();
    server.close();
  }, 10);
  return token ?? undefined;
}
