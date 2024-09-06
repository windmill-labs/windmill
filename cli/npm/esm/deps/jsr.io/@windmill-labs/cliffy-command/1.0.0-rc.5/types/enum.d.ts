import { Type } from "../type.js";
import type { ArgumentValue } from "../types.js";
/** Enum type. Allows only provided values. */
export declare class EnumType<const TValue extends string | number | boolean> extends Type<TValue> {
    private readonly allowedValues;
    constructor(values: ReadonlyArray<TValue> | Record<string, TValue>);
    parse(type: ArgumentValue): TValue;
    values(): Array<TValue>;
    complete(): Array<TValue>;
}
//# sourceMappingURL=enum.d.ts.map