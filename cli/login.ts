import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/mod.ts";
import {
  Input,
  Secret,
} from "https://deno.land/x/cliffy@v0.25.4/prompt/mod.ts";
import { GlobalOptions } from "./types.ts";
import { UserService } from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getStore } from "./store.ts";
import { getContext } from "./context.ts";

export type Options = GlobalOptions;

async function login(opts: Options, email?: string, password?: string) {
  const { urlStore } = await getContext(opts);
  email = email ?? (await Input.prompt({ message: "Input your Email" }));
  password =
    password ?? (await Secret.prompt({ message: "Input your Password" }));

  const token = await UserService.login({
    requestBody: {
      email: email,
      password: password,
    },
  });

  await Deno.writeTextFile(urlStore + "token", token);
  console.log(colors.bold.underline.green("Successfully logged in!"));
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
  .arguments("[email:string] [password:string]")
  .action(login as any);

export default command;
