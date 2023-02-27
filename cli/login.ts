import { GlobalOptions } from "./types.ts";
import { colors, getAvailablePort, Secret, Select } from "./deps.ts";
import { open } from 'https://deno.land/x/open/index.ts';

export async function loginInteractive(remote: string) {
  let token: string | undefined;
  if (
    await Select.prompt({
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
    }) === "b"
  ) {
    token = await browserLogin(remote);
  } else {
    token = await Secret.prompt("Enter your token");
  }

  return token;
}

// deno-lint-ignore require-await
export async function tryGetLoginInfo(
  opts: GlobalOptions,
): Promise<string | undefined> {
  if (opts.token) {
    return opts.token;
  }

  return undefined;
}

export async function browserLogin(
  baseUrl: string,
): Promise<string | undefined> {
  const port = await getAvailablePort();
  if (port == undefined) {
    console.log(colors.red.underline("failed to aquire port"));
    return undefined;
  }

  const server = Deno.listen({ transport: "tcp", port });
  const url = `${baseUrl}user/cli?port=${port}`
  console.log(`Login by going to ${url}`);
  try {
    await open(url)
    console.log("Opened browser for you");
  } catch {
    console.error(`Failed to open browser, please navgiate to ${url}`)
  }
  const firstConnection = await server.accept();
  const httpFirstConnection = Deno.serveHttp(firstConnection);
  const firstRequest = (await httpFirstConnection.nextRequest())!;
  const params = new URL(firstRequest.request.url!).searchParams;
  const token = params.get("token");
  // const _workspace = params.get("workspace");
  await firstRequest?.respondWith(
    Response.redirect(baseUrl + "user/cli-success", 302),
  );

  setTimeout(() => {
    httpFirstConnection.close();
    server.close();
  }, 10);
  return token ?? undefined;
}
