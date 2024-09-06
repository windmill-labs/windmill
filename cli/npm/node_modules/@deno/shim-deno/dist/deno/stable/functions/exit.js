"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.exit = void 0;
const exit = function exit(code) {
    return process.exit(code);
};
exports.exit = exit;
