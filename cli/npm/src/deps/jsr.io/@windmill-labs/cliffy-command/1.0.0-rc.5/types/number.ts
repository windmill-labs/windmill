import { number } from "../../../cliffy-flags/1.0.0-rc.5/mod.js";
import { Type } from "../type.js";
import type { ArgumentValue } from "../types.js";

/** Number type. */
export class NumberType extends Type<number> {
  /** Parse number type. */
  public parse(type: ArgumentValue): number {
    return number(type);
  }
}
