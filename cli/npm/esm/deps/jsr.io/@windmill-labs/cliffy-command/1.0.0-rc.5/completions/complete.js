import { UnknownCompletionCommandError } from "../_errors.js";
import { writeSync } from "../../../cliffy-internal/1.0.0-rc.5/runtime/write_sync.js";
import { Command } from "../command.js";
/** Execute auto completion method of command and action. */
export class CompleteCommand extends Command {
    constructor(cmd) {
        super();
        return this
            .description("Get completions for given action from given command.")
            .noGlobals()
            .arguments("<action:string> [command...:string]")
            .action(async (_, action, ...commandNames) => {
            let parent;
            const completeCommand = commandNames
                ?.reduce((cmd, name) => {
                parent = cmd;
                const childCmd = cmd.getCommand(name, false);
                if (!childCmd) {
                    throw new UnknownCompletionCommandError(name, cmd.getCommands());
                }
                return childCmd;
            }, cmd || this.getMainCommand()) ?? (cmd || this.getMainCommand());
            const completion = completeCommand
                .getCompletion(action);
            const result = await completion?.complete(completeCommand, parent) ?? [];
            if (result?.length) {
                writeSync(new TextEncoder().encode(result.join("\n")));
            }
        })
            .reset();
    }
}
