"use strict";
/// <reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.osUptime = void 0;
const os_1 = require("os");
const osUptime = function osUptime() {
    return (0, os_1.uptime)();
};
exports.osUptime = osUptime;
