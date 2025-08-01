"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const ordered_object_literal_1 = require("@stoplight/ordered-object-literal");
exports.KEYS = Symbol.for(ordered_object_literal_1.ORDER_KEY_ID);
const traps = {
    ownKeys(target) {
        return exports.KEYS in target ? target[exports.KEYS] : Reflect.ownKeys(target);
    },
};
exports.trapAccess = (target) => new Proxy(target, traps);
//# sourceMappingURL=trapAccess.js.map