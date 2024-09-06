"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.Permissions = void 0;
const PermissionStatus_js_1 = require("../classes/PermissionStatus.js");
class Permissions {
    query(desc) {
        return Promise.resolve(this.querySync(desc));
    }
    querySync(_desc) {
        return new PermissionStatus_js_1.PermissionStatus("granted");
    }
    revoke(desc) {
        return Promise.resolve(this.revokeSync(desc));
    }
    revokeSync(_desc) {
        return new PermissionStatus_js_1.PermissionStatus("denied");
    }
    request(desc) {
        return this.query(desc);
    }
    requestSync(desc) {
        return this.querySync(desc);
    }
}
exports.Permissions = Permissions;
