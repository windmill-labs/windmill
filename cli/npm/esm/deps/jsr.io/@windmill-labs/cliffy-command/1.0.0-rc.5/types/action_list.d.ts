import type { Command } from "../command.js";
import { StringType } from "./string.js";
/** Completion list type. */
export declare class ActionListType extends StringType {
    protected cmd: Command;
    constructor(cmd: Command);
    /** Complete action names. */
    complete(): string[];
}
//# sourceMappingURL=action_list.d.ts.map