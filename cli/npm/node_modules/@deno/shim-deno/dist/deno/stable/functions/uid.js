"use strict";
/// <reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
var _a;
Object.defineProperty(exports, "__esModule", { value: true });
exports.uid = void 0;
const process_1 = __importDefault(require("process"));
exports.uid = (_a = process_1.default.getuid) !== null && _a !== void 0 ? _a : (() => null);
