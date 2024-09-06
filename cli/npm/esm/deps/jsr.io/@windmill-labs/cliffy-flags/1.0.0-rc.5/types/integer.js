import { InvalidTypeError } from "../_errors.js";
/** Integer type handler. Excepts any integer value. */
export const integer = (type) => {
    const value = Number(type.value);
    if (Number.isInteger(value)) {
        return value;
    }
    throw new InvalidTypeError(type);
};
