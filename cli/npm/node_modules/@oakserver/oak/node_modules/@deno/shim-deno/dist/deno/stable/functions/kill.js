"use strict";
/// <reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.kill = void 0;
const os_1 = __importDefault(require("os"));
const process_1 = __importDefault(require("process"));
const kill = function (pid, signo) {
    if (pid < 0 && os_1.default.platform() === "win32") {
        throw new TypeError("Invalid pid");
    }
    process_1.default.kill(pid, signo);
};
exports.kill = kill;
