"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.execPath = void 0;
const which_1 = __importDefault(require("which"));
const execPath = () => which_1.default.sync("deno");
exports.execPath = execPath;
