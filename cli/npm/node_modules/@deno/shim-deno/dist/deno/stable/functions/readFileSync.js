"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.readFileSync = void 0;
const fs_1 = require("fs");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const readFileSync = function readFileSync(path) {
    try {
        const buf = (0, fs_1.readFileSync)(path);
        return new Uint8Array(buf.buffer, buf.byteOffset, buf.length);
    }
    catch (e) {
        throw (0, errorMap_js_1.default)(e);
    }
};
exports.readFileSync = readFileSync;
