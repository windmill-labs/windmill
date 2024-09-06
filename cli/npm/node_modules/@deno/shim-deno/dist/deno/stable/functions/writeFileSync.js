"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.writeFileSync = void 0;
const os_1 = require("os");
const openSync_js_1 = require("./openSync.js");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const statSync_js_1 = require("./statSync.js");
const chmodSync_js_1 = require("./chmodSync.js");
const writeFileSync = function writeFileSync(path, data, options = {}) {
    try {
        if (options.create !== undefined) {
            const create = !!options.create;
            if (!create) {
                // verify that file exists
                (0, statSync_js_1.statSync)(path);
            }
        }
        const openOptions = {
            write: true,
            create: true,
            createNew: options.createNew,
            append: !!options.append,
            truncate: !options.append,
        };
        const file = (0, openSync_js_1.openSync)(path, openOptions);
        if (options.mode !== undefined &&
            options.mode !== null &&
            (0, os_1.platform)() !== "win32") {
            (0, chmodSync_js_1.chmodSync)(path, options.mode);
        }
        let nwritten = 0;
        while (nwritten < data.length) {
            nwritten += file.writeSync(data.subarray(nwritten));
        }
        file.close();
    }
    catch (e) {
        throw (0, errorMap_js_1.default)(e);
    }
};
exports.writeFileSync = writeFileSync;
