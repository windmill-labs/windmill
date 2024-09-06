import type { Command } from "../command.js";
import { StringType } from "./string.js";

/** Completion list type. */
export class ActionListType extends StringType {
  constructor(protected cmd: Command) {
    super();
  }

  /** Complete action names. */
  public complete(): string[] {
    return this.cmd.getCompletions()
      .map((type) => type.name)
      // filter unique values
      .filter((value, index, self) => self.indexOf(value) === index);
  }
}
