// deno-lint-ignore-file no-explicit-any
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { GlobalOptions } from "./types.ts";
import { add } from "./workspace.ts";

async function setup(
  opts: GlobalOptions & {
    remote: string | undefined;
    name: string | undefined;
  }
) {
  await add(opts, opts.name, opts.remote, undefined);
  await login(opts, the);
  console.log(colors.green.bold.underline("Everything setup. Ready to use."));
}

const command = new Command()
  .description("setup windmill access")
  .option("--remote <remote:string>", "Specify the remote inline")
  .option("--name <name:string>", "Specify the remote name inline")
  .action(setup as any);

export default command;
