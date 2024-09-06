"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.readDirSync = void 0;
const fs_1 = require("fs");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const readDirSync = function* readDir(path) {
    try {
        for (const e of (0, fs_1.readdirSync)(String(path), { withFileTypes: true })) {
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
exports.readDirSync = readDirSync;
