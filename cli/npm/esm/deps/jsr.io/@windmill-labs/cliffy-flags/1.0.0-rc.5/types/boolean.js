import { InvalidTypeError } from "../_errors.js";
/** Boolean type handler. Excepts `true`, `false`, `1`, `0` */
export const boolean = (type) => {
    if (~["1", "true"].indexOf(type.value)) {
        return true;
    }
    if (~["0", "false"].indexOf(type.value)) {
        return false;
    }
    throw new InvalidTypeError(type, ["true", "false", "1", "0"]);
};
