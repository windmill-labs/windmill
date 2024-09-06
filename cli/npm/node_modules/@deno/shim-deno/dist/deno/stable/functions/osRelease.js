"use strict";
/// <reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.osRelease = void 0;
const os_1 = require("os");
const osRelease = function osRelease() {
    return (0, os_1.release)();
};
exports.osRelease = osRelease;
