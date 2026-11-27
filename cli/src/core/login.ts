import { GlobalOptions } from "../types.ts";
import { colors } from "@cliffy/ansi/colors";
import * as getPort from "get-port";
import * as log from "./log.ts";
import * as open from "open";
import { Secret } from "@cliffy/prompt/secret";
import { Select } from "@cliffy/prompt/select";
import * as http from "node:http";

/** `workspace` names the workspace the token is for, so the login page can offer to bind it. */
export async function loginInteractive(remote: string, workspace?: string) {
  let token: string | undefined;
  if (!process.stdin.isTTY) {
    log.infoStderr("Not a TTY, can't login interactively.");
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
    token = await browserLogin(remote, workspace);
  } else {
    token = await Secret.prompt("Enter your token");
  }
  return token;
}

export async function tryGetLoginInfo(
  opts: GlobalOptions
): Promise<string | undefined> {
  if (opts.token) {
    return opts.token;
  }

  return undefined;
}

export async function browserLogin(
  baseUrl: string,
  workspace?: string
): Promise<string | undefined> {
  const env =
    process.env["TOKEN_PORT"] != undefined
      ? parseInt(process.env["TOKEN_PORT"]!)
      : undefined;
  const port = await getPort.default({ port: env });

  if (port == undefined) {
    log.infoStderr(colors.red.underline("failed to aquire port"));
    return undefined;
  }

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

    const url =
      `${baseUrl}user/cli?port=${port}` +
      (workspace ? `&workspace=${encodeURIComponent(workspace)}` : "");
    log.infoStderr(`Login by going to ${url}`);

    try {
      open.default(url).catch((error) => {
        console.error(
          `Failed to open browser, please navigate to ${url}, error: ${error}`
        );
      });

      log.infoStderr("Opened browser for you");
    } catch (error) {
      console.error(
        `Failed to open browser, please navigate to ${url}, error: ${error}`
      );
    }

    // Start the server
    server.listen(port, () => {
      log.infoStderr(`Listening on port ${port}`);
    });
  });
}
