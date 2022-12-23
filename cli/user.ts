// deno-lint-ignore-file no-explicit-any
import { requireLogin } from "./context.ts";
import { GlobalOptions } from "./types.ts";
import {
  colors,
  Command,
  GlobalUserInfo,
  passwordGenerator,
  Table,
  UserService,
} from "./deps.ts";

async function list(opts: GlobalOptions) {
  await requireLogin(opts);

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
      ]),
    )
    .render();
}

async function add(
  opts: GlobalOptions & {
    superadmin?: boolean;
    company?: string;
    name?: string;
  },
  email: string,
  password?: string,
) {
  await requireLogin(opts);
  const password_final = password ?? passwordGenerator("*", 15);
  await UserService.createUserGlobally({
    requestBody: {
      email,
      password: password_final,
      super_admin: opts.superadmin ?? false,
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
  await requireLogin(opts);

  await UserService.globalUserDelete({ email });
  console.log(colors.green("Deleted User " + email));
}

async function createToken(
  opts: GlobalOptions & { email: string; password: string },
) {
  if (opts.email && opts.password) {
    console.log(
      "Token: " + await UserService.login({
        requestBody: {
          email: opts.email,
          password: opts.password,
        },
      }),
    );
  }

  await requireLogin(opts);
  console.log("Token: " + await UserService.createToken({ requestBody: {} }));
}

const command = new Command()
  .description("user related commands")
  .action(list as any)
  .command("add", "Create a user")
  .arguments("<email:string> [password:string]")
  .option("--superadmin", "Specify to make the new user superadmin.")
  .option(
    "--company <company:string>",
    "Specify to set the company of the new user.",
  )
  .option("--name <name:string>", "Specify to set the name of the new user.")
  .action(add as any)
  .command("remove", "Delete a user")
  .arguments("<email:string>")
  .action(remove as any)
  .command("create-token")
  .option(
    "--email <email:string>",
    "Specify credentials to use for authentication. This will not be stored. It will only be used to exchange for a token with the API server, which will not be stored either.",
    {
      depends: ["password"],
    },
  )
  .option(
    "--password <password:string>",
    "Specify credentials to use for authentication. This will not be stored. It will only be used to exchange for a token with the API server, which will not be stored either.",
    {
      depends: ["email"],
    },
  )
  .action(createToken as any);

export default command;
