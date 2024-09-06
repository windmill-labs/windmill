"use strict";
/// <reference path="./lib.deno.unstable.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.utimeSync = exports.utime = exports.futimeSync = exports.futime = void 0;
const fs_1 = __importDefault(require("fs"));
const errorMap_js_1 = __importDefault(require("../internal/errorMap.js"));
const variables_js_1 = require("../stable/variables.js");
const futime = async function (rid, atime, mtime) {
    try {
        await new Promise((resolve, reject) => {
            // doesn't exist in fs.promises
            fs_1.default.futimes(rid, atime, mtime, (err) => {
                if (err) {
                    reject(err);
                }
                else {
                    resolve();
                }
            });
        });
    }
    catch (error) {
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.futime = futime;
const futimeSync = function (rid, atime, mtime) {
    try {
        fs_1.default.futimesSync(rid, atime, mtime);
    }
    catch (error) {
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.futimeSync = futimeSync;
const utime = async function (path, atime, mtime) {
    try {
        await fs_1.default.promises.utimes(path, atime, mtime);
    }
    catch (error) {
        if ((error === null || error === void 0 ? void 0 : error.code) === "ENOENT") {
            throw new variables_js_1.errors.NotFound(`No such file or directory (os error 2), utime '${path}'`);
        }
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.utime = utime;
const utimeSync = function (path, atime, mtime) {
    try {
        fs_1.default.utimesSync(path, atime, mtime);
    }
    catch (error) {
        if ((error === null || error === void 0 ? void 0 : error.code) === "ENOENT") {
            throw new variables_js_1.errors.NotFound(`No such file or directory (os error 2), utime '${path}'`);
        }
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.utimeSync = utimeSync;
