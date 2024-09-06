"use strict";
/// <reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.addSignalListener = void 0;
const process_1 = __importDefault(require("process"));
function denoSignalToNodeJs(signal) {
    if (signal === "SIGEMT") {
        throw new Error("SIGEMT is not supported");
    }
    return signal;
}
const addSignalListener = (signal, handler) => {
    process_1.default.addListener(denoSignalToNodeJs(signal), handler);
};
exports.addSignalListener = addSignalListener;
