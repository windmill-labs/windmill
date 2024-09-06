"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.open = void 0;
const fs_1 = require("fs");
const util_1 = require("util");
const FsFile_js_1 = require("../classes/FsFile.js");
const fs_flags_js_1 = require("../../internal/fs_flags.js");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const nodeOpen = (0, util_1.promisify)(fs_1.open);
const open = async function open(path, { read, write, append, truncate, create, createNew, mode = 0o666 } = {
    read: true,
}) {
    const flagMode = (0, fs_flags_js_1.getFsFlag)({
        read,
        write,
        append,
        truncate,
        create,
        createNew,
    });
    try {
        const fd = await nodeOpen(path, flagMode, mode);
        return new FsFile_js_1.File(fd);
    }
    catch (err) {
        throw (0, errorMap_js_1.default)(err);
    }
};
exports.open = open;
