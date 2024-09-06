"use strict";
///<reference path="../lib.deno.d.ts" />
var _a, _b;
var _c;
Object.defineProperty(exports, "__esModule", { value: true });
exports.PermissionStatus = void 0;
// The listeners don't actually matter because the state of these permissions
// is constant and mocked as Node.js has all doors open.
(_a = (_c = globalThis).EventTarget) !== null && _a !== void 0 ? _a : (_c.EventTarget = (_b = require("events").EventTarget) !== null && _b !== void 0 ? _b : null);
class PermissionStatus extends EventTarget {
    /** @internal */
    constructor(state) {
        super();
        this.state = state;
        this.onchange = null;
        this.partial = false;
    }
}
exports.PermissionStatus = PermissionStatus;
