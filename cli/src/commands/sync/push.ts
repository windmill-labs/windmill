import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";

function stub(_opts: GlobalOptions, _dir?: string) {
  log.info(
    colors.red.underline(
      'Push is deprecated. Use "sync push --raw" instead. See <TODO_LINK_HERE> for more information.'
    )
  );
}

const command = new Command()
  .description("Push all files from a folder")
  .arguments("[dir:string]")
  .action(stub as any);

export default command;
