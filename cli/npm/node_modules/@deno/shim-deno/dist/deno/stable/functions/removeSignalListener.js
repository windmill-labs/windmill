"use strict";
/// <reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.removeSignalListener = void 0;
const process_1 = __importDefault(require("process"));
const removeSignalListener = (signal, handler) => {
    process_1.default.removeListener(signal, handler);
};
exports.removeSignalListener = removeSignalListener;
