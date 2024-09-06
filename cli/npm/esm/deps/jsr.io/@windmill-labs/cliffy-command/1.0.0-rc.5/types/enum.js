import { Type } from "../type.js";
import { InvalidTypeError } from "../../../cliffy-flags/1.0.0-rc.5/mod.js";
/** Enum type. Allows only provided values. */
export class EnumType extends Type {
    constructor(values) {
        super();
        Object.defineProperty(this, "allowedValues", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        this.allowedValues = Array.isArray(values) ? values : Object.values(values);
    }
    parse(type) {
        for (const value of this.allowedValues) {
            if (value.toString() === type.value) {
                return value;
            }
        }
        throw new InvalidTypeError(type, this.allowedValues.slice());
    }
    values() {
        return this.allowedValues.slice();
    }
    complete() {
        return this.values();
    }
}
