import { toObject, fromObject, toErrorEvent, fromErrorEvent } from '../error.js';
import Exception from '../error/Exception.js';

//@ts-ignore
globalThis["Exception"] = Exception;
Error.toObject = toObject;
Error.fromObject = fromObject;
Error.toErrorEvent = toErrorEvent;
Error.fromErrorEvent = fromErrorEvent;
Error.prototype.toJSON = function toJSON() {
    return toObject(this);
};
//# sourceMappingURL=error.js.map
