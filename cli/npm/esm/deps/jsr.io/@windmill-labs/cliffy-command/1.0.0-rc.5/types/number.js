import { number } from "../../../cliffy-flags/1.0.0-rc.5/mod.js";
import { Type } from "../type.js";
/** Number type. */
export class NumberType extends Type {
    /** Parse number type. */
    parse(type) {
        return number(type);
    }
}
