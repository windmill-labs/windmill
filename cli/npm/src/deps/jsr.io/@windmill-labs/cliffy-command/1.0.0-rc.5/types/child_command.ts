import type { Command } from "../command.js";
import { StringType } from "./string.js";

/** String type with auto completion of child command names. */
export class ChildCommandType extends StringType {
  #cmd?: Command;

  constructor(cmd?: Command) {
    super();
    this.#cmd = cmd;
  }

  /** Complete child command names. */
  public complete(cmd: Command): string[] {
    return (this.#cmd ?? cmd)?.getCommands(false)
      .map((cmd: Command) => cmd.getName()) || [];
  }
}
