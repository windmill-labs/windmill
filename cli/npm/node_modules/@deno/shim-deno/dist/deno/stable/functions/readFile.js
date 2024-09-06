"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.readFile = void 0;
const promises_1 = require("fs/promises");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const readFile = async function readFile(path, { signal } = {}) {
    try {
        const buf = await (0, promises_1.readFile)(path, { signal });
        return new Uint8Array(buf.buffer, buf.byteOffset, buf.length);
    }
    catch (e) {
        throw (0, errorMap_js_1.default)(e);
    }
};
exports.readFile = readFile;
