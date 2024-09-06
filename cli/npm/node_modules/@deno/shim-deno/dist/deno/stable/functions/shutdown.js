"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.shutdown = void 0;
const net_1 = require("net");
const shutdown = async function shutdown(rid) {
    await new Promise((resolve) => new net_1.Socket({ fd: rid }).end(resolve));
};
exports.shutdown = shutdown;
