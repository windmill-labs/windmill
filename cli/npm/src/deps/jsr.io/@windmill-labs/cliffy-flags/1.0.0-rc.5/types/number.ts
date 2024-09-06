import type { ArgumentValue, TypeHandler } from "../types.js";
import { InvalidTypeError } from "../_errors.js";

/** Number type handler. Excepts any numeric value. */
export const number: TypeHandler<number> = (type: ArgumentValue): number => {
  const value = Number(type.value);
  if (Number.isFinite(value)) {
    return value;
  }

  throw new InvalidTypeError(type);
};
