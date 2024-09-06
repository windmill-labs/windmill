"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.fstatSync = void 0;
const fs_1 = require("fs");
const stat_js_1 = require("./stat.js");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const fstatSync = function fstatSync(fd) {
    try {
        return (0, stat_js_1.denoifyFileInfo)((0, fs_1.fstatSync)(fd));
    }
    catch (err) {
        throw (0, errorMap_js_1.default)(err);
    }
};
exports.fstatSync = fstatSync;
