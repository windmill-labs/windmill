"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.readTextFile = void 0;
const promises_1 = require("fs/promises");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const readTextFile = async (path, { signal } = {}) => {
    try {
        return await (0, promises_1.readFile)(path, { encoding: "utf8", signal });
    }
    catch (e) {
        throw (0, errorMap_js_1.default)(e);
    }
};
exports.readTextFile = readTextFile;
