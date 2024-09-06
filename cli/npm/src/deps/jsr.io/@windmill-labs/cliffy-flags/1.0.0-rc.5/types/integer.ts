import type { ArgumentValue, TypeHandler } from "../types.js";
import { InvalidTypeError } from "../_errors.js";

/** Integer type handler. Excepts any integer value. */
export const integer: TypeHandler<number> = (type: ArgumentValue): number => {
  const value = Number(type.value);
  if (Number.isInteger(value)) {
    return value;
  }

  throw new InvalidTypeError(type);
};
