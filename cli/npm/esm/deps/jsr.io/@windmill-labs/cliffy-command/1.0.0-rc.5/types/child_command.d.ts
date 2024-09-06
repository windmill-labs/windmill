import type { Command } from "../command.js";
import { StringType } from "./string.js";
/** String type with auto completion of child command names. */
export declare class ChildCommandType extends StringType {
    #private;
    constructor(cmd?: Command);
    /** Complete child command names. */
    complete(cmd: Command): string[];
}
//# sourceMappingURL=child_command.d.ts.map