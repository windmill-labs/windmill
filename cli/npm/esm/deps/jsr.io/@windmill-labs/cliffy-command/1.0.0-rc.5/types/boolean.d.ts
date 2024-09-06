import type { ArgumentValue } from "../types.js";
import { Type } from "../type.js";
/** Boolean type with auto completion. Allows `true`, `false`, `0` and `1`. */
export declare class BooleanType extends Type<boolean> {
    /** Parse boolean type. */
    parse(type: ArgumentValue): boolean;
    /** Complete boolean type. */
    complete(): string[];
}
//# sourceMappingURL=boolean.d.ts.map