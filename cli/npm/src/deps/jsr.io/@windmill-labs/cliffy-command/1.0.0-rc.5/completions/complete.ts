import { UnknownCompletionCommandError } from "../_errors.js";
import { writeSync } from "../../../cliffy-internal/1.0.0-rc.5/runtime/write_sync.js";
import { Command } from "../command.js";
import type { Completion } from "../types.js";

/** Execute auto completion method of command and action. */
export class CompleteCommand extends Command<
  void,
  void,
  void,
  [action: string, ...commandNames: Array<string>]
> {
  public constructor(cmd?: Command) {
    super();
    return this
      .description(
        "Get completions for given action from given command.",
      )
      .noGlobals()
      .arguments("<action:string> [command...:string]")
      .action(async (_, action: string, ...commandNames: Array<string>) => {
        let parent: Command | undefined;
        const completeCommand: Command = commandNames
          ?.reduce((cmd: Command, name: string): Command => {
            parent = cmd;
            const childCmd: Command | undefined = cmd.getCommand(name, false);
            if (!childCmd) {
              throw new UnknownCompletionCommandError(name, cmd.getCommands());
            }
            return childCmd;
          }, cmd || this.getMainCommand()) ?? (cmd || this.getMainCommand());

        const completion: Completion | undefined = completeCommand
          .getCompletion(action);
        const result: Array<string | number | boolean> =
          await completion?.complete(completeCommand, parent) ?? [];

        if (result?.length) {
          writeSync(new TextEncoder().encode(result.join("\n")));
        }
      })
      .reset();
  }
}
