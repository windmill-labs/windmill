// deno-lint-ignore-file no-explicit-any
import { colors, Command } from "./deps.ts";
import { GlobalOptions } from "./types.ts";

async function stub(
  _opts: GlobalOptions,
  _dir?: string,
) {
  console.log(
    colors.red.underline(
      'Push is deprecated. Use "sync push --raw" instead. See <TODO_LINK_HERE> for more information.',
    ),
  );
}

const command = new Command()
  .description("Push all files from a folder")
  .arguments("[dir:string]")
  .action(stub as any);

export default command;
