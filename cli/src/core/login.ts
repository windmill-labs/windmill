import { GlobalOptions } from "../types.ts";
import { colors, getPort, log, open, Secret, Select } from "../../deps.ts";
import * as http from "node:http";

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
  const env =
    Deno.env.get("TOKEN_PORT") != undefined
      ? parseInt(Deno.env.get("TOKEN_PORT")!)
      : undefined;
  const port = await getPort.default({ port: env });

  if (port == undefined) {
    log.info(colors.red.underline("failed to aquire port"));
    return undefined;
  }

  // const server = Deno.listen({ transport: "tcp", port });
  // const url = `${baseUrl}user/cli?port=${port}`;
  // log.info(`Login by going to ${url}`);
  // try {
  //   await open.openApp(open.apps.browser, { arguments: [url] });

  //   log.info("Opened browser for you");
  // } catch {
  //   console.error(`Failed to open browser, please navigate to ${url}`);
  // }
  // const firstConnection = await server.accept();
  // const httpFirstConnection = Deno.serveHttp(firstConnection);
  // const firstRequest = (await httpFirstConnection.nextRequest())!;
  // const params = new URL(firstRequest.request.url!).searchParams;
  // const token = params.get("token");
  // // const _workspace = params.get("workspace");
  // await firstRequest?.respondWith(
  //   Response.redirect(baseUrl + "user/cli-success", 302)
  // );

  // setTimeout(() => {
  //   httpFirstConnection.close();
  //   server.close();
  // }, 10);
  // return token ?? undefined;

  return new Promise((resolve) => {
    const server = http.createServer((req, res) => {
      const params = new URL(req.url!, `http://${req.headers.host}`)
        .searchParams;
      const token = params.get("token");

      // Redirect the user to the success page
      res.writeHead(302, { Location: `${baseUrl}user/cli-success` });
      res.end();

      // Close the server after a short delay
      setTimeout(() => {
        server.close();
      }, 10);

      // Resolve the promise with the token
      resolve(token ?? undefined);
    });

    const url = `${baseUrl}user/cli?port=${port}`;
    log.info(`Login by going to ${url}`);

    try {
      open.openApp(open.apps.browser, { arguments: [url] }).catch((error) => {
        console.error(
          `Failed to open browser, please navigate to ${url}, error: ${error}`
        );
      });

      log.info("Opened browser for you");
    } catch (error) {
      console.error(
        `Failed to open browser, please navigate to ${url}, error: ${error}`
      );
    }

    // Start the server
    server.listen(port, () => {
      console.log(`Listening on port ${port}`);
    });
  });
}
