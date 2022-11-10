import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/table.ts";
import { UserService } from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { GlobalUserInfo } from "https://deno.land/x/windmill@v1.41.0/windmill-api/index.ts";
import { passwordGenerator } from "https://deno.land/x/password_generator@latest/mod.ts"; // TODO: I think the version is called latest, but it's still pinned.
import { getContext } from "./context.ts";
import { GlobalOptions } from "./types.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";

async function list(opts: GlobalOptions) {
  const _ = await getContext(opts);

  const perPage = 10;
  let page = 0;
  const total: GlobalUserInfo[] = [];
  while (true) {
    const res = await UserService.listUsersAsSuperAdmin({ page, perPage });
    total.push(...res);
    page += 1;

    if (res.length < perPage) {
      break;
    }
  }

  new Table()
    .header(["email", "name", "company", "verified", "super admin"])
    .padding(2)
    .border(true)
    .body(
      total.map((x) => [
        x.email,
        x.name ?? "-",
        x.company ?? "-",
        x.verified ? "true" : "false",
        x.super_admin ? "true" : "false",
      ])
    )
    .render();
}

async function add(
  opts: GlobalOptions & {
    superAdmin?: boolean;
    company?: string;
    name?: string;
  },
  email: string,
  password?: string
) {
  const _ = await getContext(opts);
  const password_final = password ?? passwordGenerator("*", 15);
  await UserService.createUserGlobally({
    requestBody: {
      email,
      password: password_final,
      super_admin: opts.superAdmin ?? false,
      company: opts.company,
      name: opts.name,
    },
  });
  if (!password) {
    console.log(colors.red("New Password: " + password_final));
  }
  console.log(colors.underline.green("New User Created."));
}
async function remove(opts: GlobalOptions, email: string) {
  const _ = await getContext(opts);
  throw new Error("API unsupported");
}

const command = new Command()
  .description("user related commands")
  .action(list as any)
  .command("add", "Create a user")
  .arguments("<email:string> [password:string]")
  .option("--superadmin", "Specify to make the new user superadmin.")
  .option(
    "--company <company:string>",
    "Specify to set the company of the new user."
  )
  .option("--name <name:string>", "Specify to set the name of the new user.")
  .action(add as any)
  .command("remove", "Delete a user")
  .arguments("<email:string>")
  .action(remove as any);

export default command;
