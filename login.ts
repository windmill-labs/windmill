import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/mod.ts";
import {
  Input,
  Secret,
} from "https://deno.land/x/cliffy@v0.25.4/prompt/mod.ts";
import { GlobalOptions } from "./types.ts";
import dir from "https://deno.land/x/dir@1.5.1/mod.ts";
import {
  setClient,
  UserService,
} from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";

export type Options = GlobalOptions;

function hash_string(str: string): number {
  let hash = 0,
    i,
    chr;
  if (str.length === 0) return hash;
  for (i = 0; i < str.length; i++) {
    chr = str.charCodeAt(i);
    hash = (hash << 5) - hash + chr;
    hash |= 0; // Convert to 32bit integer
  }
  return hash;
}

async function login({ baseUrl }: Options, email?: string, password?: string) {
  setClient("no_token", baseUrl);
  const baseHash = Math.abs(hash_string(baseUrl)).toString(16);
  const store = dir("config") + "/windmill/";
  try {
    await Deno.mkdir(store);
  } catch {
    /* ignore existing folder */
  }
  const baseStore = store + baseHash + "/";
  try {
    await Deno.mkdir(baseStore);
  } catch {
    /*ignore existing folder */
  }

  email = email ?? (await Input.prompt({ message: "Input your Email" }));
  password =
    password ?? (await Secret.prompt({ message: "Input your Password" }));

  const token = await UserService.login({
    requestBody: {
      email: email,
      password: password,
    },
  });

  await Deno.writeTextFile(baseStore + "token", token);
  console.log(colors.bold.underline.green("Successfully logged in!"));
}

const command = new Command()
  .description("log into windmill")
  .arguments("[email:string] [password:string]")
  .action(login as any);

export default command;
