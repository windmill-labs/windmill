import { Type } from "../type.js";
import { integer } from "../../../cliffy-flags/1.0.0-rc.5/mod.js";
/** Integer type. */
export class IntegerType extends Type {
    /** Parse integer type. */
    parse(type) {
        return integer(type);
    }
}
