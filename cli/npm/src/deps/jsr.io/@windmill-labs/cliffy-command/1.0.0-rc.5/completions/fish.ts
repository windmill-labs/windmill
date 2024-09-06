import { Command } from "../command.js";
import { dim, italic } from "../../../../@std/fmt/0.225.6/colors.js";
import { FishCompletionsGenerator } from "./_fish_completions_generator.js";

/** Generates fish completions script. */
export class FishCompletionsCommand
  extends Command<void, void, { name: string }> {
  readonly #cmd?: Command;

  public constructor(cmd?: Command) {
    super();
    this.#cmd = cmd;
    return this
      .description(() => {
        const baseCmd = this.#cmd || this.getMainCommand();
        return `Generate shell completions for fish.

To enable fish completions for this program add following line to your ${
          dim(italic("~/.config/fish/config.fish"))
        }:

    ${dim(italic(`source (${baseCmd.getPath()} completions fish | psub)`))}`;
      })
      .noGlobals()
      .option("-n, --name <command-name>", "The name of the main command.")
      .action(({ name = this.getMainCommand().getName() }) => {
        const baseCmd = this.#cmd || this.getMainCommand();
        console.log(FishCompletionsGenerator.generate(name, baseCmd));
      });
  }
}
