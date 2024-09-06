"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.readDir = void 0;
const promises_1 = require("fs/promises");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const readDir = async function* readDir(path) {
    try {
        for await (const e of await (0, promises_1.opendir)(String(path))) {
            const ent = {
                name: e.name,
                isFile: e.isFile(),
                isDirectory: e.isDirectory(),
                isSymlink: e.isSymbolicLink(),
            };
            yield ent;
        }
    }
    catch (e) {
        throw (0, errorMap_js_1.default)(e);
    }
};
exports.readDir = readDir;
