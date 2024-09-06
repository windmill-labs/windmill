"use strict";
///<reference path="../lib.deno.d.ts" />
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.stat = exports.denoifyFileInfo = void 0;
const promises_1 = require("node:fs/promises");
const os = __importStar(require("node:os"));
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const isWindows = os.platform() === "win32";
function denoifyFileInfo(s) {
    return {
        atime: s.atime,
        birthtime: s.birthtime,
        blksize: isWindows ? null : s.blksize,
        blocks: isWindows ? null : s.blocks,
        dev: s.dev,
        gid: isWindows ? null : s.gid,
        ino: isWindows ? null : s.ino,
        isDirectory: s.isDirectory(),
        isFile: s.isFile(),
        isSymlink: s.isSymbolicLink(),
        isBlockDevice: isWindows ? null : s.isBlockDevice(),
        isCharDevice: isWindows ? null : s.isCharacterDevice(),
        isFifo: isWindows ? null : s.isFIFO(),
        isSocket: isWindows ? null : s.isSocket(),
        mode: isWindows ? null : s.mode,
        mtime: s.mtime,
        nlink: isWindows ? null : s.nlink,
        rdev: isWindows ? null : s.rdev,
        size: s.size,
        uid: isWindows ? null : s.uid,
    };
}
exports.denoifyFileInfo = denoifyFileInfo;
const stat = async (path) => {
    try {
        return denoifyFileInfo(await (0, promises_1.stat)(path));
    }
    catch (e) {
        throw (0, errorMap_js_1.default)(e);
    }
};
exports.stat = stat;
