import { Select } from "https://deno.land/x/cliffy@v0.25.4/prompt/select.ts";
import { GlobalOptions } from "./types.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getAvailablePort } from "https://deno.land/x/port@1.0.0/mod.ts";
import { Secret } from "https://deno.land/x/cliffy@v0.25.4/prompt/secret.ts";

export async function loginInteractive(remote: string) {
  let token: string | undefined;
  if (
    await Select.prompt({
      message: "How do you wanna login",
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
  console.log(`Login by going to ${baseUrl}user/cli?port=${port}`);
  const firstConnection = await server.accept();
  const httpFirstConnection = Deno.serveHttp(firstConnection);
  const firstRequest = (await httpFirstConnection.nextRequest())!;
  const params = new URL(firstRequest.request.url!).searchParams;
  const token = params.get("token");
  const _workspace = params.get("workspace");
  await firstRequest?.respondWith(
    Response.redirect(baseUrl + "suser/cli-success", 302),
  );

  setTimeout(() => {
    httpFirstConnection.close();
    server.close();
  }, 10);
  return token ?? undefined;
}
