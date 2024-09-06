import { string } from "../../../cliffy-flags/1.0.0-rc.5/mod.js";
import { Type } from "../type.js";
/** String type. Allows any value. */
export class StringType extends Type {
    /** Complete string type. */
    parse(type) {
        return string(type);
    }
}
